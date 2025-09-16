use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;
use tokio::time::timeout;

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
            min_new_tokens: 1,
            top_k: 50,
            top_p: 1.0,
            repetition_penalty: 1.1,
            stop_sequences: vec!["\n\n".to_string(), "Human:".to_string(), "Assistant:".to_string()],
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
        
        // Improved response parsing with better error handling
        for line in response_text.lines() {
            if line.starts_with("data: ") {
                let json_data = &line[6..]; // Remove "data: " prefix
                
                // Skip empty data lines
                if json_data.trim().is_empty() || json_data.trim() == "[DONE]" {
                    continue;
                }
                
                match serde_json::from_str::<GenerationData>(json_data) {
                    Ok(data) => {
                        if let Some(result) = data.results.first() {
                            answer.push_str(&result.generated_text);
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
            return Err(anyhow::anyhow!("Empty response from WatsonX API"));
        }
        
        Ok(answer.trim().to_string())
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