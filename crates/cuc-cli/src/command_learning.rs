//! Command learning engine for capturing and learning from user corrections

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;
use chrono::Utc;
use cuc_core::{Error, Result, CommandLearning};

/// Type of correction made by the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrectionType {
    PluginMissing,
    SyntaxError,
    WrongCommand,
    ParameterError,
    Other,
}

/// Command learning engine
pub struct CommandLearningEngine {
    corrections: HashMap<String, CommandLearning>,
    file_path: String,
}

impl CommandLearningEngine {
    /// Create a new command learning engine
    pub fn new(file_path: &str) -> Result<Self> {
        let mut engine = Self {
            corrections: HashMap::new(),
            file_path: file_path.to_string(),
        };

        // Try to load existing corrections
        if Path::new(file_path).exists() {
            if let Err(e) = engine.load_sync() {
                eprintln!("Warning: Failed to load corrections: {}", e);
            }
        }

        Ok(engine)
    }

    /// Load corrections synchronously (for initialization)
    fn load_sync(&mut self) -> Result<()> {
        let content = std::fs::read_to_string(&self.file_path)
            .map_err(|e| Error::Io(e))?;

        let corrections: Vec<CommandLearning> = serde_json::from_str(&content)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        for correction in corrections {
            self.corrections.insert(correction.query.clone(), correction);
        }

        Ok(())
    }

    /// Load corrections from file
    pub async fn load(&mut self) -> Result<()> {
        let content = fs::read_to_string(&self.file_path)
            .await
            .map_err(|e| Error::Io(e))?;

        let corrections: Vec<CommandLearning> = serde_json::from_str(&content)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        for correction in corrections {
            self.corrections.insert(correction.query.clone(), correction);
        }

        Ok(())
    }

    /// Save corrections to file
    pub async fn save(&self) -> Result<()> {
        let corrections: Vec<&CommandLearning> = self.corrections.values().collect();
        let json = serde_json::to_string_pretty(&corrections)
            .map_err(|e| Error::Serialization(e.to_string()))?;

        fs::write(&self.file_path, json)
            .await
            .map_err(|e| Error::Io(e))?;

        Ok(())
    }

    /// Add a correction
    pub async fn add_correction(
        &mut self,
        query: String,
        correct_command: String,
        error_pattern: Option<String>,
    ) -> Result<()> {
        let learning = CommandLearning {
            query: query.clone(),
            correct_command,
            error_pattern,
            timestamp: Utc::now().timestamp(),
        };

        self.corrections.insert(query, learning);
        self.save().await?;

        Ok(())
    }

    /// Get a learned command for a query
    pub fn get_learned_command(&self, query: &str) -> Option<&CommandLearning> {
        self.corrections.get(query)
    }

    /// Get all corrections
    pub fn get_all_corrections(&self) -> Vec<&CommandLearning> {
        self.corrections.values().collect()
    }

    /// Find similar corrections based on query similarity
    pub fn find_similar(&self, query: &str, threshold: f32) -> Vec<&CommandLearning> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<(&CommandLearning, f32)> = self
            .corrections
            .values()
            .map(|learning| {
                let similarity = self.calculate_similarity(&query_lower, &learning.query.to_lowercase());
                (learning, similarity)
            })
            .filter(|(_, score)| *score >= threshold)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.into_iter().map(|(learning, _)| learning).collect()
    }

    /// Simple word-based similarity calculation
    fn calculate_similarity(&self, query1: &str, query2: &str) -> f32 {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_command_learning() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let mut engine = CommandLearningEngine::new(path).unwrap();

        engine
            .add_correction(
                "list databases".to_string(),
                "ibmcloud resource service-instances".to_string(),
                None,
            )
            .await
            .unwrap();

        let learned = engine.get_learned_command("list databases");
        assert!(learned.is_some());
    }
}
