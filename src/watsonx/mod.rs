use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tokio::time::timeout;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WatsonxAI {
    pub api_key: String,
    pub project_id: String,
    pub access_token: Option<String>,
    pub iam_url: String,
    client: Client,
}

#[derive(Serialize)]
struct TokenRequest {
    grant_type: String,
    apikey: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Serialize)]
struct GenerationParams {
    decoding_method: String,
    max_new_tokens: u32,
    min_new_tokens: u32,
    top_k: u32,
    top_p: f32,
    repetition_penalty: f32,
    stop_sequences: Vec<String>,
}

#[derive(Serialize)]
struct GenerationRequest {
    input: String,
    parameters: GenerationParams,
    model_id: String,
    project_id: String,
}

#[derive(Deserialize)]
struct GenerationResults {
    generated_text: String,
}

#[derive(Deserialize)]
struct GenerationData {
    results: Vec<GenerationResults>,
}

#[derive(Debug, Clone)]
pub struct GenerationAttempt {
    pub prompt: String,
    pub result: String,
    pub quality_score: f32,
    pub attempt_number: u32,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_timeout: Duration,
    pub enable_progressive_prompts: bool,
    pub quality_threshold: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_timeout: Duration::from_secs(30),
            enable_progressive_prompts: true,
            quality_threshold: 0.7,
        }
    }
}

// Model constants
impl WatsonxAI {
    pub const GRANITE_3_3_8B_INSTRUCT: &'static str = "ibm/granite-3-3-8b-instruct";

    pub fn new() -> Result<Self> {
        let api_key = env::var("WATSONX_API_KEY")
            .or_else(|_| env::var("API_KEY"))
            .map_err(|_| anyhow::anyhow!("WATSONX_API_KEY or API_KEY environment variable not found"))?;
        let project_id = env::var("WATSONX_PROJECT_ID")
            .or_else(|_| env::var("PROJECT_ID"))
            .map_err(|_| anyhow::anyhow!("WATSONX_PROJECT_ID or PROJECT_ID environment variable not found"))?;
        let iam_url = env::var("IAM_IBM_CLOUD_URL")
            .unwrap_or_else(|_| "iam.cloud.ibm.com".to_string());

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .danger_accept_invalid_certs(true) // Match Python's verify=False
            .build()?;

        Ok(WatsonxAI {
            api_key,
            project_id,
            access_token: None,
            iam_url,
            client,
        })
    }

    pub async fn connect(&mut self) -> Result<()> {
        let token_request = TokenRequest {
            grant_type: "urn:ibm:params:oauth:grant-type:apikey".to_string(),
            apikey: self.api_key.clone(),
        };

        let url = format!("https://{}/identity/token", self.iam_url);
        
        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&token_request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Authentication failed: {}", response.status()));
        }

        let token_response: TokenResponse = response.json().await?;
        self.access_token = Some(token_response.access_token);

        Ok(())
    }

    pub async fn watsonx_gen(
        &self,
        prompt: &str,
        model_id: &str,
        max_output: u32,
    ) -> Result<String> {
        self.watsonx_gen_with_timeout(prompt, model_id, max_output, Duration::from_secs(60))
            .await
    }

    /// Generate with retry mechanism and feedback integration
    pub async fn watsonx_gen_with_feedback(
        &self,
        base_prompt: &str,
        model_id: &str,
        max_output: u32,
        previous_failures: &[String],
        retry_config: Option<RetryConfig>,
    ) -> Result<GenerationAttempt> {
        let config = retry_config.unwrap_or_default();
        let mut best_attempt: Option<GenerationAttempt> = None;
        
        for attempt in 1..=config.max_attempts {
            let enhanced_prompt = self.enhance_prompt_with_feedback(
                base_prompt, 
                previous_failures, 
                attempt
            );
            
            let timeout_duration = config.base_timeout + Duration::from_secs((attempt - 1) as u64 * 10);
            
            match self.watsonx_gen_with_timeout(
                &enhanced_prompt,
                model_id,
                max_output,
                timeout_duration,
            ).await {
                Ok(result) => {
                    let quality_score = self.assess_generation_quality(&result, base_prompt);
                    
                    let current_attempt = GenerationAttempt {
                        prompt: enhanced_prompt,
                        result: result.clone(),
                        quality_score,
                        attempt_number: attempt,
                    };
                    
                    // If quality is good enough, return immediately
                    if quality_score >= config.quality_threshold {
                        return Ok(current_attempt);
                    }
                    
                    // Keep track of best attempt so far
                    if best_attempt.as_ref().map_or(true, |best| quality_score > best.quality_score) {
                        best_attempt = Some(current_attempt);
                    }
                }
                Err(e) => {
                    if attempt == config.max_attempts {
                        return Err(e);
                    }
                    // Continue to next attempt on error
                }
            }
        }
        
        best_attempt.ok_or_else(|| anyhow::anyhow!("All generation attempts failed"))
    }

    pub async fn watsonx_gen_with_timeout(
        &self,
        prompt: &str,
        model_id: &str,
        max_output: u32,
        timeout_duration: Duration,
    ) -> Result<String> {
        let generation_future = self.perform_generation(prompt, model_id, max_output);
        
        match timeout(timeout_duration, generation_future).await {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("Request timed out")),
        }
    }

    async fn perform_generation(
        &self,
        prompt: &str,
        model_id: &str,
        max_output: u32,
    ) -> Result<String> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Not authenticated. Call connect() first."))?;

        // Enhanced parameters following WatsonX best practices
        let params = GenerationParams {
            decoding_method: "greedy".to_string(),
            max_new_tokens: max_output,
            min_new_tokens: 5, // Ensure we get at least some meaningful output
            top_k: 50,
            top_p: 1.0,
            repetition_penalty: 1.1,
            stop_sequences: vec!["Human:".to_string(), "Assistant:".to_string(), "Query:".to_string()],
        };

        let request_body = GenerationRequest {
            input: prompt.to_string(),
            parameters: params,
            model_id: model_id.to_string(),
            project_id: self.project_id.clone(),
        };

        let url = "https://us-south.ml.cloud.ibm.com/ml/v1/text/generation_stream?version=2023-05-29";

        let response = self
            .client
            .post(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&request_body)
            .send()
            .await?;

        // Enhanced error handling with detailed status information
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "WatsonX API request failed with status {}: {}",
                status,
                error_text
            ));
        }

        let mut answer = String::new();
        let response_text = response.text().await?;
        
        // Improved response parsing for Server-Sent Events (SSE) format
        for line in response_text.lines() {
            // Handle SSE format: look for lines starting with "data: "
            if line.starts_with("data: ") {
                let json_data = &line[6..]; // Remove "data: " prefix
                
                // Skip empty data lines
                if json_data.trim().is_empty() || json_data.trim() == "[DONE]" {
                    continue;
                }
                
                match serde_json::from_str::<GenerationData>(json_data) {
                    Ok(data) => {
                        if let Some(result) = data.results.first() {
                            let generated_text = &result.generated_text;
                            // Only add non-empty text that's not just whitespace
                            if !generated_text.trim().is_empty() {
                                answer.push_str(generated_text);
                            }
                        }
                    }
                    Err(e) => {
                        // Log parsing errors but continue processing
                        eprintln!("Warning: Failed to parse response line: {} - Error: {}", json_data, e);
                    }
                }
            }
        }
        
        // Ensure we have some response
        if answer.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty response from WatsonX API. Raw response: {}", response_text));
        }
        
        // Clean up the response by extracting just the command part
        let mut cleaned_answer = answer.trim().to_string();
        
        // Remove any prefixes like "Answer:" or similar
        if cleaned_answer.starts_with("Answer:") {
            cleaned_answer = cleaned_answer.strip_prefix("Answer:").unwrap_or(&cleaned_answer).trim().to_string();
        }
        
        // Remove any suffixes like "Query:" or similar that might appear due to stop sequence issues
        if let Some(query_pos) = cleaned_answer.find("Query:") {
            cleaned_answer = cleaned_answer[..query_pos].trim().to_string();
        }
        
        // Take only the first line to ensure we get just the command
        let final_answer = cleaned_answer
            .lines()
            .next()
            .unwrap_or(&cleaned_answer)
            .trim()
            .to_string();

        Ok(final_answer)
    }

    /// Enhance prompt with feedback from previous failures
    fn enhance_prompt_with_feedback(
        &self,
        base_prompt: &str,
        previous_failures: &[String],
        attempt_number: u32,
    ) -> String {
        if previous_failures.is_empty() {
            return base_prompt.to_string();
        }

        let mut enhanced_prompt = base_prompt.to_string();
        
        // Add failure context
        enhanced_prompt.push_str("\n\nPREVIOUS ATTEMPTS FAILED WITH THESE ERRORS:\n");
        for (i, failure) in previous_failures.iter().enumerate() {
            enhanced_prompt.push_str(&format!("{}. {}\n", i + 1, failure));
        }
        
        // Add progressive guidance based on attempt number
        match attempt_number {
            1 => {
                enhanced_prompt.push_str("\nPlease generate a more specific and accurate IBM Cloud CLI command.");
            }
            2 => {
                enhanced_prompt.push_str("\nIMPORTANT: The previous command failed. Please:\n");
                enhanced_prompt.push_str("- Check command syntax carefully\n");
                enhanced_prompt.push_str("- Verify subcommand names\n");
                enhanced_prompt.push_str("- Ensure proper parameter format\n");
                enhanced_prompt.push_str("- Consider if plugins are required\n");
            }
            _ => {
                enhanced_prompt.push_str("\nCRITICAL: Multiple attempts failed. Please:\n");
                enhanced_prompt.push_str("- Use only well-established IBM Cloud CLI commands\n");
                enhanced_prompt.push_str("- Avoid deprecated or experimental features\n");
                enhanced_prompt.push_str("- Consider alternative approaches\n");
                enhanced_prompt.push_str("- Focus on core IBM Cloud services\n");
            }
        }
        
        enhanced_prompt
    }

    /// Assess the quality of generated command
    fn assess_generation_quality(&self, result: &str, _original_prompt: &str) -> f32 {
        let mut score = 0.0;
        let mut max_score = 0.0;
        
        // Check if result starts with ibmcloud
        max_score += 0.3;
        if result.trim().starts_with("ibmcloud") {
            score += 0.3;
        }
        
        // Check if result is not empty and reasonable length
        max_score += 0.2;
        let trimmed = result.trim();
        if !trimmed.is_empty() && trimmed.len() > 8 && trimmed.len() < 200 {
            score += 0.2;
        }
        
        // Check for common IBM Cloud CLI patterns
        max_score += 0.2;
        let common_patterns = ["resource", "service", "target", "login", "plugin", "cf", "ks", "cr"];
        if common_patterns.iter().any(|pattern| result.contains(pattern)) {
            score += 0.2;
        }
        
        // Check if it doesn't contain obvious errors
        max_score += 0.15;
        let error_indicators = ["error", "failed", "invalid", "unknown", "not found"];
        if !error_indicators.iter().any(|indicator| result.to_lowercase().contains(indicator)) {
            score += 0.15;
        }
        
        // Check for proper command structure (no multiple commands)
        max_score += 0.15;
        let line_count = result.lines().filter(|line| !line.trim().is_empty()).count();
        if line_count == 1 {
            score += 0.15;
        }
        
        // Normalize score
        if max_score > 0.0 {
            score / max_score
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_watsonx_creation() {
        // This test will fail if environment variables are not set
        // but it tests the basic structure
        std::env::set_var("API_KEY", "test_key");
        std::env::set_var("PROJECT_ID", "test_project");
        
        let result = WatsonxAI::new();
        assert!(result.is_ok());
        
        std::env::remove_var("API_KEY");
        std::env::remove_var("PROJECT_ID");
    }
}