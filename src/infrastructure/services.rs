//! Infrastructure implementations of domain services

use crate::domain::{
    Command, QualityAnalysis, NaturalLanguageQuery, CloudProvider, CommandLearning,
    CommandQualityService, CommandTranslationService, CommandLearningService,
    repositories::CommandLearningRepository,
};
use crate::cli::QualityAnalyzer;
use crate::cli::CommandTranslator;
use crate::core::{LLMProvider, RAGEngine};

/// Infrastructure implementation of CommandQualityService
pub struct QualityAnalyzerService {
    analyzer: QualityAnalyzer,
}

impl QualityAnalyzerService {
    pub fn new() -> Self {
        Self {
            analyzer: QualityAnalyzer::new(),
        }
    }
}

impl CommandQualityService for QualityAnalyzerService {
    fn analyze(&self, command: &Command) -> QualityAnalysis {
        let analysis = self.analyzer.analyze(&command.value);
        QualityAnalysis::new(analysis.score, analysis.issues, analysis.suggestions)
    }
}

/// Infrastructure implementation of CommandTranslationService
pub struct CommandTranslatorService<L: LLMProvider, R: RAGEngine> {
    translator: CommandTranslator<L, R>,
}

impl<L: LLMProvider, R: RAGEngine> CommandTranslatorService<L, R> {
    pub fn new(translator: CommandTranslator<L, R>) -> Self {
        Self { translator }
    }
}

#[async_trait::async_trait]
impl<L: LLMProvider + Send + Sync, R: RAGEngine + Send + Sync> CommandTranslationService for CommandTranslatorService<L, R> {
    async fn translate(
        &self,
        query: &NaturalLanguageQuery,
        provider: CloudProvider,
    ) -> Result<Command, String> {
        // For now, delegate to existing translator
        // TODO: Enhance with provider-specific logic
        let command_str = self.translator.translate(query.as_str()).await
            .map_err(|e| e.to_string())?;
        
        let mut command = Command::new(command_str, provider);
        
        // Analyze quality
        let quality_service = QualityAnalyzerService::new();
        let analysis = quality_service.analyze(&command);
        command.update_quality(analysis.score, analysis.issues);
        
        Ok(command)
    }
}

/// Infrastructure implementation of CommandLearningService
pub struct CommandLearningServiceImpl<R: CommandLearningRepository> {
    repository: R,
}

impl<R: CommandLearningRepository> CommandLearningServiceImpl<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl<R: CommandLearningRepository + Send + Sync> CommandLearningService for CommandLearningServiceImpl<R> {
    async fn learn_from_correction(
        &mut self,
        query: &NaturalLanguageQuery,
        correct_command: &Command,
        error_pattern: Option<String>,
    ) -> Result<(), String> {
        let learning = CommandLearning::new(
            query.as_str().to_string(),
            correct_command.value.clone(),
            error_pattern,
        );
        
        self.repository.save(learning).await
    }

    async fn find_similar(
        &self,
        query: &NaturalLanguageQuery,
        threshold: f32,
    ) -> Vec<Command> {
        // This would need async, but for now return empty
        // TODO: Make this async or use blocking
        Vec::new()
    }
}

