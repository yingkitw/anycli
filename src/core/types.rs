//! Common types used across the IBM Cloud CLI AI system

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for retry behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Represents a generation attempt with quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationAttempt {
    pub prompt: String,
    pub result: String,
    pub quality_score: f32,
    pub attempt_number: u32,
}

/// Command learning entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandLearning {
    pub query: String,
    pub correct_command: String,
    pub error_pattern: Option<String>,
    pub timestamp: i64,
}

/// Quality analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAnalysis {
    pub score: f32,
    pub issues: Vec<String>,
    pub suggestions: Vec<String>,
}
