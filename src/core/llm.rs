//! LLM provider trait and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{Error, Result};
use super::types::{RetryConfig, GenerationAttempt};

/// Configuration for text generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    pub model_id: String,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub top_k: Option<u32>,
    pub stop_sequences: Vec<String>,
    pub timeout: Duration,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            model_id: "ibm/granite-4-h-small".to_string(),
            max_tokens: 200,
            temperature: None,
            top_p: Some(1.0),
            top_k: Some(50),
            stop_sequences: vec![
                "Human:".to_string(),
                "Assistant:".to_string(),
                "Query:".to_string(),
            ],
            timeout: Duration::from_secs(60),
        }
    }
}

/// Result of a text generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub text: String,
    pub model_id: String,
    pub tokens_used: Option<u32>,
    pub quality_score: Option<f32>,
}

/// Trait for LLM providers (e.g., WatsonX, OpenAI, etc.)
///
/// This trait defines the interface for interacting with Large Language Models.
/// It supports both simple generation and advanced generation with retry logic
/// and quality assessment.
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Connect/authenticate with the LLM provider
    async fn connect(&mut self) -> Result<()>;

    /// Generate text using the LLM with default configuration
    async fn generate(&self, prompt: &str) -> Result<GenerationResult>;

    /// Generate text with custom configuration
    async fn generate_with_config(
        &self,
        prompt: &str,
        config: &GenerationConfig,
    ) -> Result<GenerationResult>;

    /// Generate with retry mechanism and feedback integration
    async fn generate_with_feedback(
        &self,
        base_prompt: &str,
        config: &GenerationConfig,
        previous_failures: &[String],
        retry_config: Option<RetryConfig>,
    ) -> Result<GenerationAttempt>;

    /// Generate text with streaming support
    async fn generate_stream(
        &self,
        prompt: &str,
        config: &GenerationConfig,
    ) -> Result<GenerationResult>;

    /// Assess the quality of generated text
    fn assess_quality(&self, text: &str, prompt: &str) -> f32;

    /// Get the model ID being used
    fn model_id(&self) -> &str;
}
