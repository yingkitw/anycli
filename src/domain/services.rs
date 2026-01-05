//! Domain services - business logic that doesn't belong to a single entity

use crate::domain::{Command, QualityAnalysis, NaturalLanguageQuery, CloudProvider};

#[async_trait::async_trait]

/// Domain service for analyzing command quality
pub trait CommandQualityService {
    /// Analyze the quality of a command
    fn analyze(&self, command: &Command) -> QualityAnalysis;
    
    /// Check if a command is valid
    fn is_valid(&self, command: &Command) -> bool {
        let analysis = self.analyze(command);
        analysis.is_acceptable()
    }
}

/// Domain service for command translation
#[async_trait::async_trait]
pub trait CommandTranslationService {
    /// Translate a natural language query to a command
    async fn translate(
        &self,
        query: &NaturalLanguageQuery,
        provider: CloudProvider,
    ) -> Result<Command, String>;
}

/// Domain service for command learning
#[async_trait::async_trait]
pub trait CommandLearningService {
    /// Learn from a correction
    async fn learn_from_correction(
        &mut self,
        query: &NaturalLanguageQuery,
        correct_command: &Command,
        error_pattern: Option<String>,
    ) -> Result<(), String>;

    /// Find similar learned commands
    async fn find_similar(
        &self,
        query: &NaturalLanguageQuery,
        threshold: f32,
    ) -> Vec<Command>;
}

