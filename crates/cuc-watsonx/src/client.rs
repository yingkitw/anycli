//! WatsonX AI client implementation

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;

use cuc_core::{
    LLMProvider, GenerationConfig, GenerationResult, GenerationAttempt,
    RetryConfig, Error, Result,
};

use crate::config::WatsonxConfig;

/// WatsonX AI client
pub struct WatsonxClient {
    config: WatsonxConfig,
    access_token: Option<String>,
    client: Client,
    current_model: String,
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

impl WatsonxClient {
    /// Model constants
    pub const GRANITE_4_H_SMALL: &'static str = "ibm/granite-4-h-small";
    pub const GRANITE_3_3_8B_INSTRUCT: &'static str = "ibm/granite-3-3-8b-instruct";

    /// Create a new WatsonX client from configuration
    pub fn new(config: WatsonxConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;

        Ok(Self {
            config,
            access_token: None,
            client,
            current_model: Self::GRANITE_4_H_SMALL.to_string(),
        })
    }

    /// Create a new WatsonX client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = WatsonxConfig::from_env()?;
        Self::new(config)
    }

    /// Set the model to use for generation
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.current_model = model_id.into();
        self
    }

    /// Perform the actual generation request
    async fn perform_generation(
        &self,
        prompt: &str,
        config: &GenerationConfig,
    ) -> Result<String> {
        let access_token = self
            .access_token
            .as_ref()
            .ok_or_else(|| Error::Authentication("Not authenticated. Call connect() first.".to_string()))?;

        let params = GenerationParams {
            decoding_method: "greedy".to_string(),
            max_new_tokens: config.max_tokens,
            min_new_tokens: 5,
            top_k: config.top_k.unwrap_or(50),
            top_p: config.top_p.unwrap_or(1.0),
            repetition_penalty: 1.1,
            stop_sequences: config.stop_sequences.clone(),
        };

        let request_body = GenerationRequest {
            input: prompt.to_string(),
            parameters: params,
            model_id: config.model_id.clone(),
            project_id: self.config.project_id.clone(),
        };

        let url = format!(
            "{}/ml/v1/text/generation_stream?version=2023-05-29",
            self.config.api_url
        );

        let response = self
            .client
            .post(&url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
            .json(&request_body)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::LLMProvider(format!(
                "WatsonX API request failed with status {}: {}",
                status, error_text
            )));
        }

        let mut answer = String::new();
        let response_text = response
            .text()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        // Parse Server-Sent Events (SSE) format
        for line in response_text.lines() {
            if line.starts_with("data: ") {
                let json_data = &line[6..];

                if json_data.trim().is_empty() || json_data.trim() == "[DONE]" {
                    continue;
                }

                match serde_json::from_str::<GenerationData>(json_data) {
                    Ok(data) => {
                        if let Some(result) = data.results.first() {
                            let generated_text = &result.generated_text;
                            if !generated_text.trim().is_empty() {
                                answer.push_str(generated_text);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse response line: {} - Error: {}", json_data, e);
                    }
                }
            }
        }

        if answer.trim().is_empty() {
            return Err(Error::LLMProvider(format!(
                "Empty response from WatsonX API. Raw response: {}",
                response_text
            )));
        }

        // Clean up the response
        let mut cleaned_answer = answer.trim().to_string();

        if cleaned_answer.starts_with("Answer:") {
            cleaned_answer = cleaned_answer
                .strip_prefix("Answer:")
                .unwrap_or(&cleaned_answer)
                .trim()
                .to_string();
        }

        if let Some(query_pos) = cleaned_answer.find("Query:") {
            cleaned_answer = cleaned_answer[..query_pos].trim().to_string();
        }

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

        enhanced_prompt.push_str("\n\nPREVIOUS ATTEMPTS FAILED WITH THESE ERRORS:\n");
        for (i, failure) in previous_failures.iter().enumerate() {
            enhanced_prompt.push_str(&format!("{}. {}\n", i + 1, failure));
        }

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
}

#[async_trait]
impl LLMProvider for WatsonxClient {
    async fn connect(&mut self) -> Result<()> {
        let token_request = TokenRequest {
            grant_type: "urn:ibm:params:oauth:grant-type:apikey".to_string(),
            apikey: self.config.api_key.clone(),
        };

        let url = format!("https://{}/identity/token", self.config.iam_url);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&token_request)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(Error::Authentication(format!(
                "Authentication failed: {}",
                response.status()
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| Error::Serialization(e.to_string()))?;

        self.access_token = Some(token_response.access_token);

        Ok(())
    }

    async fn generate(&self, prompt: &str) -> Result<GenerationResult> {
        let config = GenerationConfig {
            model_id: self.current_model.clone(),
            ..Default::default()
        };
        self.generate_with_config(prompt, &config).await
    }

    async fn generate_with_config(
        &self,
        prompt: &str,
        config: &GenerationConfig,
    ) -> Result<GenerationResult> {
        let generation_future = self.perform_generation(prompt, config);

        let text = match timeout(config.timeout, generation_future).await {
            Ok(result) => result?,
            Err(_) => return Err(Error::Timeout("Request timed out".to_string())),
        };

        Ok(GenerationResult {
            text,
            model_id: config.model_id.clone(),
            tokens_used: None,
            quality_score: None,
        })
    }

    async fn generate_with_feedback(
        &self,
        base_prompt: &str,
        config: &GenerationConfig,
        previous_failures: &[String],
        retry_config: Option<RetryConfig>,
    ) -> Result<GenerationAttempt> {
        let retry_cfg = retry_config.unwrap_or_default();
        let mut best_attempt: Option<GenerationAttempt> = None;

        for attempt in 1..=retry_cfg.max_attempts {
            let enhanced_prompt = self.enhance_prompt_with_feedback(
                base_prompt,
                previous_failures,
                attempt,
            );

            let timeout_duration = retry_cfg.base_timeout + Duration::from_secs((attempt - 1) as u64 * 10);

            let mut attempt_config = config.clone();
            attempt_config.timeout = timeout_duration;

            match self.generate_with_config(&enhanced_prompt, &attempt_config).await {
                Ok(result) => {
                    let quality_score = self.assess_quality(&result.text, base_prompt);

                    let current_attempt = GenerationAttempt {
                        prompt: enhanced_prompt,
                        result: result.text.clone(),
                        quality_score,
                        attempt_number: attempt,
                    };

                    if quality_score >= retry_cfg.quality_threshold {
                        return Ok(current_attempt);
                    }

                    if best_attempt.as_ref().map_or(true, |best| quality_score > best.quality_score) {
                        best_attempt = Some(current_attempt);
                    }
                }
                Err(e) => {
                    if attempt == retry_cfg.max_attempts {
                        return Err(e);
                    }
                }
            }
        }

        best_attempt.ok_or_else(|| Error::LLMProvider("All generation attempts failed".to_string()))
    }

    async fn generate_stream(
        &self,
        prompt: &str,
        config: &GenerationConfig,
    ) -> Result<GenerationResult> {
        // For now, use the same implementation as generate_with_config
        // In the future, this could be enhanced to support true streaming
        self.generate_with_config(prompt, config).await
    }

    fn assess_quality(&self, text: &str, _prompt: &str) -> f32 {
        let mut score = 0.0;
        let mut max_score = 0.0;

        // Check if result starts with ibmcloud
        max_score += 0.3;
        if text.trim().starts_with("ibmcloud") {
            score += 0.3;
        }

        // Check if result is not empty and reasonable length
        max_score += 0.2;
        let trimmed = text.trim();
        if !trimmed.is_empty() && trimmed.len() > 8 && trimmed.len() < 200 {
            score += 0.2;
        }

        // Check for common IBM Cloud CLI patterns
        max_score += 0.2;
        let common_patterns = ["resource", "service", "target", "login", "plugin", "cf", "ks", "cr"];
        if common_patterns.iter().any(|pattern| text.contains(pattern)) {
            score += 0.2;
        }

        // Check if it doesn't contain obvious errors
        max_score += 0.15;
        let error_indicators = ["error", "failed", "invalid", "unknown", "not found"];
        if !error_indicators.iter().any(|indicator| text.to_lowercase().contains(indicator)) {
            score += 0.15;
        }

        // Check for proper command structure (no multiple commands)
        max_score += 0.15;
        let line_count = text.lines().filter(|line| !line.trim().is_empty()).count();
        if line_count == 1 {
            score += 0.15;
        }

        if max_score > 0.0 {
            score / max_score
        } else {
            0.0
        }
    }

    fn model_id(&self) -> &str {
        &self.current_model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_assessment() {
        let config = WatsonxConfig::new("test_key".to_string(), "test_project".to_string());
        let client = WatsonxClient::new(config).unwrap();

        let good_command = "ibmcloud resource groups";
        let score = client.assess_quality(good_command, "list resource groups");
        assert!(score > 0.5);

        let bad_command = "error: invalid command";
        let score = client.assess_quality(bad_command, "list resource groups");
        assert!(score < 0.5);
    }
}
