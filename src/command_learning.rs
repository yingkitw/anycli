use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCorrection {
    pub original_query: String,
    pub incorrect_command: String,
    pub correct_command: String,
    pub error_message: Option<String>,
    pub correction_type: CorrectionType,
    pub timestamp: DateTime<Utc>,
    pub confidence_score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CorrectionType {
    CommandNotFound,
    InvalidSyntax,
    MissingPlugin,
    WrongSubcommand,
    ParameterError,
    Other(String),
    CommandFix,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LearningDatabase {
    corrections: Vec<CommandCorrection>,
    patterns: HashMap<String, Vec<String>>, // Common error patterns -> corrections
    last_updated: DateTime<Utc>,
}

pub struct CommandLearningEngine {
    database: LearningDatabase,
    database_path: String,
}

impl CommandLearningEngine {
    pub fn new(database_path: &str) -> Result<Self> {
        let database = if Path::new(database_path).exists() {
            let content = fs::read_to_string(database_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| LearningDatabase::new())
        } else {
            LearningDatabase::new()
        };
        
        Ok(Self {
            database,
            database_path: database_path.to_string(),
        })
    }
    
    /// Add a command correction to the learning database
    pub fn add_correction(
        &mut self,
        original_query: &str,
        incorrect_command: &str,
        correct_command: &str,
        error_message: Option<&str>,
        correction_type: CorrectionType,
    ) -> Result<()> {
        let correction = CommandCorrection {
            original_query: original_query.to_string(),
            incorrect_command: incorrect_command.to_string(),
            correct_command: correct_command.to_string(),
            error_message: error_message.map(|s| s.to_string()),
            correction_type: correction_type.clone(),
            timestamp: Utc::now(),
            confidence_score: 1.0, // Start with high confidence for manual corrections
        };
        
        self.database.corrections.push(correction);
        self.update_patterns(&correction_type, incorrect_command, correct_command);
        self.database.last_updated = Utc::now();
        self.save()?;
        
        println!("📚 Learned correction: '{}' -> '{}'", incorrect_command, correct_command);
        Ok(())
    }
    
    /// Get suggestions based on learned corrections
    pub fn get_suggestions(&self, failed_command: &str, error_message: Option<&str>) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Look for exact query matches
        for correction in &self.database.corrections {
            if correction.original_query.to_lowercase().contains(&failed_command.to_lowercase()) ||
               correction.incorrect_command == failed_command {
                suggestions.push(correction.correct_command.clone());
            }
        }
        
        // Look for pattern matches
        for (pattern, corrections) in &self.database.patterns {
            if failed_command.contains(pattern) {
                suggestions.extend(corrections.clone());
            }
        }
        
        // Remove duplicates and sort by relevance
        suggestions.sort();
        suggestions.dedup();
        suggestions.truncate(3); // Limit to top 3 suggestions
        
        suggestions
    }
    
    /// Analyze error message and suggest correction type
    pub fn analyze_error(&self, error_message: &str) -> CorrectionType {
        let error_lower = error_message.to_lowercase();
        
        if error_lower.contains("not a registered command") || error_lower.contains("command not found") {
            CorrectionType::CommandNotFound
        } else if error_lower.contains("invalid syntax") || error_lower.contains("usage:") {
            CorrectionType::InvalidSyntax
        } else if error_lower.contains("plugin") && error_lower.contains("not installed") {
            CorrectionType::MissingPlugin
        } else if error_lower.contains("subcommand") {
            CorrectionType::WrongSubcommand
        } else if error_lower.contains("parameter") || error_lower.contains("argument") {
            CorrectionType::ParameterError
        } else {
            CorrectionType::Other(error_message.to_string())
        }
    }
    
    /// Get learning context for RAG system
    pub fn get_learning_context(&self, query: &str) -> String {
        let mut context = String::new();
        
        // Add relevant corrections as context
        let relevant_corrections: Vec<_> = self.database.corrections
            .iter()
            .filter(|c| {
                c.original_query.to_lowercase().contains(&query.to_lowercase()) ||
                query.to_lowercase().contains(&c.original_query.to_lowercase())
            })
            .take(3)
            .collect();
        
        if !relevant_corrections.is_empty() {
            context.push_str("\nLearned corrections:\n");
            for correction in relevant_corrections {
                context.push_str(&format!(
                    "- Query: '{}' -> Correct command: '{}'\n",
                    correction.original_query,
                    correction.correct_command
                ));
            }
        }
        
        // Add common patterns
        if query.contains("services") {
            context.push_str("\nNote: 'ibmcloud services' is not valid. Use 'ibmcloud resource service-instances' instead.\n");
        }
        
        context
    }
    
    /// Get statistics about learned corrections
    pub fn get_stats(&self) -> (usize, usize, DateTime<Utc>) {
        (
            self.database.corrections.len(),
            self.database.patterns.len(),
            self.database.last_updated,
        )
    }
    
    /// Update pattern database based on correction
    fn update_patterns(&mut self, correction_type: &CorrectionType, incorrect: &str, correct: &str) {
        let pattern_key = match correction_type {
            CorrectionType::CommandNotFound => {
                // Extract the problematic part
                if let Some(parts) = incorrect.strip_prefix("ibmcloud ") {
                    if let Some(first_word) = parts.split_whitespace().next() {
                        first_word.to_string()
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                }
            }
            _ => "general".to_string(),
        };
        
        self.database.patterns
            .entry(pattern_key)
            .or_insert_with(Vec::new)
            .push(correct.to_string());
    }
    
    /// Save the learning database to disk
    fn save(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(&self.database)?;
        fs::write(&self.database_path, json)?;
        Ok(())
    }
}

impl LearningDatabase {
    fn new() -> Self {
        Self {
            corrections: Vec::new(),
            patterns: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

/// Helper function to detect if a command failure might be correctable
pub fn is_correctable_error(error_message: &str) -> bool {
    let error_lower = error_message.to_lowercase();
    error_lower.contains("not a registered command") ||
    error_lower.contains("command not found") ||
    error_lower.contains("invalid syntax") ||
    error_lower.contains("plugin") ||
    error_lower.contains("subcommand")
}

/// Extract command name from error message for better learning
pub fn extract_failed_command(error_message: &str) -> Option<String> {
    // Look for patterns like "'services' is not a registered command"
    if let Some(start) = error_message.find("'") {
        if let Some(end) = error_message[start + 1..].find("'") {
            return Some(error_message[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_command_learning_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let engine = CommandLearningEngine::new(temp_file.path().to_str().unwrap());
        assert!(engine.is_ok());
    }
    
    #[test]
    fn test_error_analysis() {
        let temp_file = NamedTempFile::new().unwrap();
        let engine = CommandLearningEngine::new(temp_file.path().to_str().unwrap()).unwrap();
        
        let error_type = engine.analyze_error("'services' is not a registered command");
        matches!(error_type, CorrectionType::CommandNotFound);
    }
    
    #[test]
    fn test_extract_failed_command() {
        let error = "'services' is not a registered command";
        let extracted = extract_failed_command(error);
        assert_eq!(extracted, Some("services".to_string()));
    }
}