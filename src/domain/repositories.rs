//! Repository interfaces - abstract data access

use crate::domain::{CommandLearning, NaturalLanguageQuery};

/// Repository for command learning data
#[async_trait::async_trait]
pub trait CommandLearningRepository {
    /// Save a learning entry
    async fn save(&mut self, learning: CommandLearning) -> Result<(), String>;

    /// Find learning by query
    async fn find_by_query(&self, query: &NaturalLanguageQuery) -> Option<CommandLearning>;

    /// Find all learning entries
    async fn find_all(&self) -> Vec<CommandLearning>;

    /// Find similar queries
    async fn find_similar(
        &self,
        query: &NaturalLanguageQuery,
        threshold: f32,
    ) -> Vec<CommandLearning>;
}

