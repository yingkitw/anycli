//! Value objects - immutable domain concepts

use serde::{Deserialize, Serialize};

/// Quality analysis result - value object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QualityAnalysis {
    /// Quality score (0.0 to 1.0)
    pub score: f32,
    /// Issues found
    pub issues: Vec<String>,
    /// Suggestions for improvement
    pub suggestions: Vec<String>,
}

impl QualityAnalysis {
    /// Create a new quality analysis
    pub fn new(score: f32, issues: Vec<String>, suggestions: Vec<String>) -> Self {
        Self {
            score,
            issues,
            suggestions,
        }
    }

    /// Check if the quality is acceptable
    pub fn is_acceptable(&self) -> bool {
        self.score >= 0.6 && self.issues.is_empty()
    }
}

/// Natural language query - value object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NaturalLanguageQuery {
    /// The query text
    pub text: String,
}

impl NaturalLanguageQuery {
    /// Create a new query
    pub fn new(text: String) -> Self {
        Self { text }
    }

    /// Get the query text
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl From<String> for NaturalLanguageQuery {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&str> for NaturalLanguageQuery {
    fn from(text: &str) -> Self {
        Self::new(text.to_string())
    }
}

