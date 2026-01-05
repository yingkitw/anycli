//! Use cases - application-level business operations

use crate::domain::{
    Command, NaturalLanguageQuery, CloudProvider,
    CommandTranslationService, CommandQualityService, CommandLearningService,
};

/// Use case: Translate natural language to CLI command
pub struct TranslateCommandUseCase<'a, T: CommandTranslationService> {
    translation_service: &'a T,
}

impl<'a, T: CommandTranslationService> TranslateCommandUseCase<'a, T> {
    pub fn new(translation_service: &'a T) -> Self {
        Self { translation_service }
    }

    pub async fn execute(
        &self,
        query: &NaturalLanguageQuery,
        provider: CloudProvider,
    ) -> Result<Command, String> {
        self.translation_service.translate(query, provider).await
    }
}

/// Use case: Analyze command quality
pub struct AnalyzeCommandQualityUseCase<'a, Q: CommandQualityService> {
    quality_service: &'a Q,
}

impl<'a, Q: CommandQualityService> AnalyzeCommandQualityUseCase<'a, Q> {
    pub fn new(quality_service: &'a Q) -> Self {
        Self { quality_service }
    }

    pub fn execute(&self, command: &Command) -> crate::domain::QualityAnalysis {
        self.quality_service.analyze(command)
    }
}

/// Use case: Learn from command correction
pub struct LearnFromCorrectionUseCase<'a, L: CommandLearningService> {
    learning_service: &'a mut L,
}

impl<'a, L: CommandLearningService> LearnFromCorrectionUseCase<'a, L> {
    pub fn new(learning_service: &'a mut L) -> Self {
        Self { learning_service }
    }

    pub async fn execute(
        &mut self,
        query: &NaturalLanguageQuery,
        correct_command: &Command,
        error_pattern: Option<String>,
    ) -> Result<(), String> {
        self.learning_service
            .learn_from_correction(query, correct_command, error_pattern)
            .await
    }
}

