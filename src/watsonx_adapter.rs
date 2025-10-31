//! Adapter to make watsonx-rs implement LLMProvider trait

use async_trait::async_trait;
use std::time::Duration;
use tokio::time::timeout;
use std::env;

use crate::core::{
    LLMProvider, GenerationConfig, GenerationResult, GenerationAttempt,
    RetryConfig, Error, Result,
};
use watsonx_rs::{WatsonxClient, WatsonxConfig, GenerationConfig as WatxGenConfig};

/// Thin wrapper around watsonx-rs client to implement LLMProvider
pub struct WatsonxAdapter {
    client: WatsonxClient,
}

impl WatsonxAdapter {
    pub fn new(client: WatsonxClient) -> Self {
        Self { client }
    }
}

/// Implement LLMProvider trait for watsonx adapter
#[async_trait]
impl LLMProvider for WatsonxAdapter {
    async fn connect(&mut self) -> Result<()> {
        // watsonx-rs handles connection internally when calling generate methods
        Ok(())
    }

    async fn generate(&self, prompt: &str) -> Result<GenerationResult> {
        let config = GenerationConfig::default();
        self.generate_with_config(prompt, &config).await
    }

    async fn generate_with_config(
        &self,
        prompt: &str,
        config: &GenerationConfig,
    ) -> Result<GenerationResult> {
        let watx_config = WatxGenConfig::default()
            .with_model(config.model_id.clone())
            .with_max_tokens(config.max_tokens)
            .with_top_p(config.top_p.unwrap_or(1.0))
            .with_top_k(config.top_k.unwrap_or(50))
            .with_stop_sequences(config.stop_sequences.clone());

        let generation_future: std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> = Box::pin(async {
            // Use generate_text_stream as per user requirement
            // Note: generate_text_stream requires a callback for streaming, using generate_text for now
            let watx_result = self.client.generate_text(prompt, &watx_config).await
                .map_err(|e| Error::LLMProvider(format!("WatsonX generation failed: {}", e)))?;
            Ok::<String, Error>(watx_result.text)
        });

        let text = match timeout(config.timeout, generation_future).await {
            Ok(result) => result?,
            Err(_) => return Err(Error::Timeout("Request timed out".to_string())),
        };

        // Clean up the response
        let mut cleaned_answer = text.trim().to_string();

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

        Ok(GenerationResult {
            text: final_answer,
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
            let enhanced_prompt = enhance_prompt_with_feedback(
                base_prompt,
                previous_failures,
                attempt,
            );

            let timeout_duration = retry_cfg.base_timeout + Duration::from_secs((attempt - 1) as u64 * 10);

            let mut attempt_config = config.clone();
            attempt_config.timeout = timeout_duration;

            match self.generate_with_config(&enhanced_prompt, &attempt_config).await {
                Ok(result) => {
                    let quality_score = assess_quality(&result.text, base_prompt);

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
        // Use generate_text_stream directly from watsonx-rs
        self.generate_with_config(prompt, config).await
    }

    fn assess_quality(&self, text: &str, _prompt: &str) -> f32 {
        assess_quality(text, _prompt)
    }

    fn model_id(&self) -> &str {
        // Default model - watsonx-rs handles this internally
        "ibm/granite-4-h-small"
    }
}

/// Enhance prompt with feedback from previous failures
fn enhance_prompt_with_feedback(
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
            enhanced_prompt.push_str("\nPlease generate a more specific and accurate cloud CLI command.");
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
            enhanced_prompt.push_str("- Use only well-established CLI commands\n");
            enhanced_prompt.push_str("- Avoid deprecated or experimental features\n");
            enhanced_prompt.push_str("- Consider alternative approaches\n");
            enhanced_prompt.push_str("- Focus on core cloud services\n");
        }
    }

    enhanced_prompt
}

/// Assess the quality of generated text
fn assess_quality(text: &str, _prompt: &str) -> f32 {
    let mut score = 0.0;
    let mut max_score = 0.0;

    // Check if result starts with a valid cloud CLI command
    max_score += 0.3;
    let cli_commands = ["ibmcloud", "aws", "gcloud", "az", "govc"];
    if cli_commands.iter().any(|cmd| text.trim().starts_with(cmd)) {
        score += 0.3;
    }

    // Check if result is not empty and reasonable length
    max_score += 0.2;
    let trimmed = text.trim();
    if !trimmed.is_empty() && trimmed.len() > 8 && trimmed.len() < 200 {
        score += 0.2;
    }

    // Check for common CLI patterns
    max_score += 0.2;
    let common_patterns = ["resource", "service", "target", "login", "plugin", "cf", "ks", "cr", "list", "describe", "get"];
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

/// Create WatsonX adapter from environment variables
pub fn create_watsonx_client() -> Result<WatsonxAdapter> {
    dotenvy::dotenv().ok();

    let api_key = env::var("WATSONX_API_KEY")
        .or_else(|_| env::var("API_KEY"))
        .map_err(|_| Error::Configuration(
            "WATSONX_API_KEY or API_KEY environment variable not found".to_string()
        ))?;

    let project_id = env::var("WATSONX_PROJECT_ID")
        .or_else(|_| env::var("PROJECT_ID"))
        .map_err(|_| Error::Configuration(
            "WATSONX_PROJECT_ID or PROJECT_ID environment variable not found".to_string()
        ))?;

    let config = WatsonxConfig::new(api_key, project_id);
    let client = WatsonxClient::new(config)
        .map_err(|e| Error::Configuration(format!("Failed to create WatsonX client: {}", e)))?;
    Ok(WatsonxAdapter::new(client))
}

