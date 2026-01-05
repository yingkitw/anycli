//! Repository implementations

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use serde::{Deserialize, Serialize};

use crate::domain::{
    CommandLearningRepository, CommandLearning, NaturalLanguageQuery,
};

/// File-based implementation of CommandLearningRepository
pub struct FileCommandLearningRepository {
    corrections: HashMap<String, CommandLearning>,
    file_path: String,
}

impl FileCommandLearningRepository {
    /// Create a new file-based repository
    pub fn new(file_path: &str) -> Result<Self, String> {
        let mut repo = Self {
            corrections: HashMap::new(),
            file_path: file_path.to_string(),
        };

        // Try to load existing corrections
        if Path::new(file_path).exists() {
            if let Err(e) = repo.load_sync() {
                eprintln!("Warning: Failed to load corrections: {}", e);
            }
        }

        Ok(repo)
    }

    /// Load corrections synchronously (for initialization)
    fn load_sync(&mut self) -> Result<(), String> {
        let content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| format!("IO error: {}", e))?;

        let corrections: Vec<CommandLearning> = serde_json::from_str(&content)
            .map_err(|e| format!("Deserialization error: {}", e))?;

        for correction in corrections {
            self.corrections.insert(correction.query.clone(), correction);
        }

        Ok(())
    }
}

#[async_trait]
impl CommandLearningRepository for FileCommandLearningRepository {
    async fn save(&mut self, learning: CommandLearning) -> Result<(), String> {
        self.corrections.insert(learning.query.clone(), learning.clone());
        
        let corrections: Vec<&CommandLearning> = self.corrections.values().collect();
        let json = serde_json::to_string_pretty(&corrections)
            .map_err(|e| format!("Serialization error: {}", e))?;

        fs::write(&self.file_path, json)
            .await
            .map_err(|e| format!("IO error: {}", e))?;

        Ok(())
    }

    async fn find_by_query(&self, query: &NaturalLanguageQuery) -> Option<CommandLearning> {
        self.corrections.get(query.as_str()).cloned()
    }

    async fn find_all(&self) -> Vec<CommandLearning> {
        self.corrections.values().cloned().collect()
    }

    async fn find_similar(
        &self,
        query: &NaturalLanguageQuery,
        threshold: f32,
    ) -> Vec<CommandLearning> {
        let query_lower = query.as_str().to_lowercase();
        let mut results: Vec<(CommandLearning, f32)> = self
            .corrections
            .values()
            .map(|learning| {
                let similarity = calculate_similarity(&query_lower, &learning.query.to_lowercase());
                (learning.clone(), similarity)
            })
            .filter(|(_, score)| *score >= threshold)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.into_iter().map(|(learning, _)| learning).collect()
    }
}

/// Simple word-based similarity calculation
fn calculate_similarity(query1: &str, query2: &str) -> f32 {
    let words1: Vec<&str> = query1.split_whitespace().collect();
    let words2: Vec<&str> = query2.split_whitespace().collect();

    let mut matches = 0;
    for word in &words1 {
        if words2.contains(word) {
            matches += 1;
        }
    }

    if words1.is_empty() {
        0.0
    } else {
        matches as f32 / words1.len() as f32
    }
}

