use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::fs;
use std::path::Path;
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandCorrection {
    pub original_query: String,
    pub incorrect_command: String,
    pub correct_command: String,
    pub error_message: Option<String>,
    pub correction_type: CorrectionType,
    pub timestamp: DateTime<Utc>,
    pub confidence_score: f32,
    pub success_rate: f32,
    pub usage_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CorrectionType {
    CommandNotFound,
    InvalidSyntax,
    MissingPlugin,
    WrongSubcommand,
    ParameterError,
    AuthenticationError,
    NetworkError,
    ResourceNotFound,
    PermissionDenied,
    Other(String),
    CommandFix,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub pattern_id: String,
    pub error_regex: String,
    pub common_causes: Vec<String>,
    pub suggested_fixes: Vec<String>,
    pub confidence: f32,
    pub occurrence_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    pub strategy_type: RetryStrategyType,
    pub max_attempts: u32,
    pub delay_ms: u64,
    pub success_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RetryStrategyType {
    ImmediateRetry,
    ExponentialBackoff,
    LinearBackoff,
    ContextualRetry,
    NoRetry,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LearningDatabase {
    corrections: Vec<CommandCorrection>,
    patterns: HashMap<String, Vec<String>>, // Common error patterns -> corrections
    failure_patterns: Vec<FailurePattern>,
    retry_strategies: HashMap<CorrectionType, RetryStrategy>,
    success_metrics: HashMap<String, f32>, // Command -> success rate
    last_updated: DateTime<Utc>,
}

pub struct CommandLearningEngine {
    database: LearningDatabase,
    database_path: String,
    error_patterns: Vec<Regex>,
}

impl CommandLearningEngine {
    pub fn new(database_path: &str) -> Result<Self> {
        let database = if Path::new(database_path).exists() {
            let content = fs::read_to_string(database_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| LearningDatabase::new())
        } else {
            LearningDatabase::new()
        };
        
        let mut engine = Self {
            database,
            database_path: database_path.to_string(),
            error_patterns: Vec::new(),
        };
        
        engine.initialize_error_patterns();
        engine.initialize_retry_strategies();
        Ok(engine)
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
            success_rate: 1.0,
            usage_count: 1,
        };
        
        self.database.corrections.push(correction);
        self.update_patterns(&correction_type, incorrect_command, correct_command);
        self.database.last_updated = Utc::now();
        self.save()?;
        
        println!("📚 Learned correction: '{}' -> '{}'", incorrect_command, correct_command);
        Ok(())
    }
    
    /// Get suggestions based on learned corrections
    pub fn get_suggestions(&self, failed_command: &str, _error_message: Option<&str>) -> Vec<String> {
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
    
    /// Initialize error pattern recognition
    fn initialize_error_patterns(&mut self) {
        let patterns = vec![
            r"command '([^']+)' not found",
            r"Unknown command: ([^\s]+)",
            r"Invalid syntax.*near '([^']+)'",
            r"Plugin '([^']+)' not installed",
            r"Authentication failed",
            r"Network error|Connection refused|timeout",
            r"Resource '([^']+)' not found",
            r"Permission denied|Access denied|Forbidden",
            r"Missing required parameter: ([^\s]+)",
            r"Invalid parameter value: ([^\s]+)",
        ];
        
        self.error_patterns = patterns
            .into_iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();
    }
    
    /// Initialize retry strategies for different error types
    fn initialize_retry_strategies(&mut self) {
        let strategies = vec![
            (CorrectionType::NetworkError, RetryStrategy {
                strategy_type: RetryStrategyType::ExponentialBackoff,
                max_attempts: 3,
                delay_ms: 1000,
                success_rate: 0.7,
            }),
            (CorrectionType::AuthenticationError, RetryStrategy {
                strategy_type: RetryStrategyType::NoRetry,
                max_attempts: 1,
                delay_ms: 0,
                success_rate: 0.1,
            }),
            (CorrectionType::InvalidSyntax, RetryStrategy {
                strategy_type: RetryStrategyType::ContextualRetry,
                max_attempts: 2,
                delay_ms: 500,
                success_rate: 0.8,
            }),
            (CorrectionType::CommandNotFound, RetryStrategy {
                strategy_type: RetryStrategyType::ImmediateRetry,
                max_attempts: 2,
                delay_ms: 0,
                success_rate: 0.6,
            }),
            (CorrectionType::ParameterError, RetryStrategy {
                strategy_type: RetryStrategyType::LinearBackoff,
                max_attempts: 2,
                delay_ms: 500,
                success_rate: 0.75,
            }),
        ];
        
        for (error_type, strategy) in strategies {
            self.database.retry_strategies.insert(error_type, strategy);
        }
    }
    
    /// Analyze failure patterns and suggest retry strategies
    pub fn analyze_failure_pattern(&self, error_message: &str, _command: &str) -> Option<RetryStrategy> {
        let correction_type = self.analyze_error(error_message);
        
        // Check if we have a specific retry strategy for this error type
        if let Some(strategy) = self.database.retry_strategies.get(&correction_type) {
            return Some(strategy.clone());
        }
        
        // Analyze error message patterns
        for pattern in &self.error_patterns {
            if pattern.is_match(error_message) {
                return Some(self.get_default_retry_strategy(&correction_type));
            }
        }
        
        // Default strategy for unknown errors
        Some(RetryStrategy {
            strategy_type: RetryStrategyType::LinearBackoff,
            max_attempts: 2,
            delay_ms: 1000,
            success_rate: 0.5,
        })
    }
    
    /// Get intelligent retry suggestions based on failure analysis
    pub fn get_retry_suggestions(&self, failed_command: &str, error_message: &str, attempt_count: u32) -> Vec<String> {
        let mut suggestions = Vec::new();
        let correction_type = self.analyze_error(error_message);
        
        // Get basic suggestions from existing method
        suggestions.extend(self.get_suggestions(failed_command, Some(error_message)));
        
        // Add context-specific suggestions based on error type and attempt count
        match correction_type {
            CorrectionType::AuthenticationError => {
                suggestions.push("Try running 'ibmcloud login' first".to_string());
                suggestions.push("Check your API key or credentials".to_string());
            },
            CorrectionType::NetworkError => {
                if attempt_count < 2 {
                    suggestions.push("Retry the command (network issue detected)".to_string());
                }
                suggestions.push("Check your internet connection".to_string());
            },
            CorrectionType::MissingPlugin => {
                if let Some(plugin) = self.extract_plugin_name(error_message) {
                    suggestions.push(format!("Install the plugin: ibmcloud plugin install {}", plugin));
                }
            },
            CorrectionType::ResourceNotFound => {
                suggestions.push("Verify the resource name and region".to_string());
                suggestions.push("List available resources first".to_string());
            },
            CorrectionType::ParameterError => {
                suggestions.push("Check parameter syntax and required values".to_string());
                suggestions.push("Use --help to see valid parameters".to_string());
            },
            _ => {}
        }
        
        // Remove duplicates and limit suggestions
        suggestions.sort();
        suggestions.dedup();
        suggestions.into_iter().take(5).collect()
    }
    
    /// Update success metrics for commands
    pub fn update_success_metrics(&mut self, command: &str, was_successful: bool) {
        let current_rate = self.database.success_metrics.get(command).unwrap_or(&0.5);
        let new_rate = if was_successful {
            (current_rate + 0.1).min(1.0)
        } else {
            (current_rate - 0.1).max(0.0)
        };
        
        self.database.success_metrics.insert(command.to_string(), new_rate);
        self.database.last_updated = Utc::now();
        
        // Save updated metrics
        let _ = self.save();
    }
    
    /// Get command success rate
    pub fn get_success_rate(&self, command: &str) -> f32 {
        self.database.success_metrics.get(command).unwrap_or(&0.5).clone()
    }
    
    fn get_default_retry_strategy(&self, correction_type: &CorrectionType) -> RetryStrategy {
        match correction_type {
            CorrectionType::NetworkError => RetryStrategy {
                strategy_type: RetryStrategyType::ExponentialBackoff,
                max_attempts: 3,
                delay_ms: 1000,
                success_rate: 0.7,
            },
            CorrectionType::AuthenticationError => RetryStrategy {
                strategy_type: RetryStrategyType::NoRetry,
                max_attempts: 1,
                delay_ms: 0,
                success_rate: 0.1,
            },
            _ => RetryStrategy {
                strategy_type: RetryStrategyType::LinearBackoff,
                max_attempts: 2,
                delay_ms: 500,
                success_rate: 0.6,
            },
        }
    }
    
    fn extract_plugin_name(&self, error_message: &str) -> Option<String> {
        // Try to extract plugin name from error messages
        if let Some(regex) = Regex::new(r"plugin '([^']+)'").ok() {
            if let Some(captures) = regex.captures(error_message) {
                return captures.get(1).map(|m| m.as_str().to_string());
            }
        }
        None
    }
    
    /// Get database statistics
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
            failure_patterns: Vec::new(),
            retry_strategies: HashMap::new(),
            success_metrics: HashMap::new(),
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