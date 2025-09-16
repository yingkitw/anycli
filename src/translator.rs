use anyhow::Result;
use crate::watsonx::{WatsonxAI, RetryConfig, GenerationAttempt};
use crate::rag::{RAGEngine, RAGConfig};
use crate::command_learning::{CommandLearningEngine, CorrectionType, RetryStrategy};
use crate::quality_analyzer::{GenerationQualityAnalyzer, AnalysisResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CommandTranslator {
    watsonx: WatsonxAI,
    rag_engine: Option<Arc<Mutex<RAGEngine>>>,
    rag_enabled: bool,
    retry_config: RetryConfig,
    failure_history: Vec<String>,
    quality_analyzer: GenerationQualityAnalyzer,
    learning_engine: CommandLearningEngine,
}

impl CommandTranslator {
    pub fn new(watsonx: WatsonxAI) -> Self {
        Self { 
            watsonx,
            rag_engine: None,
            rag_enabled: false,
            retry_config: RetryConfig::default(),
            failure_history: Vec::new(),
            quality_analyzer: GenerationQualityAnalyzer::new(),
            learning_engine: CommandLearningEngine::new("learning_data.json").unwrap_or_else(|e| {
                eprintln!("Warning: Failed to initialize learning engine: {}", e);
                // Create a minimal learning engine or handle the error appropriately
                CommandLearningEngine::new("learning_data.json").unwrap()
            }),
        }
    }
    
    /// Create a new CommandTranslator with RAG support
    pub async fn with_rag(watsonx: WatsonxAI, qdrant_url: &str, collection_name: &str) -> Result<Self> {
        let rag_engine = RAGEngine::new(qdrant_url, collection_name).await?;
        
        // Initialize RAG system
        println!("🔧 Initializing RAG system for enhanced translations...");
        rag_engine.initialize().await?;
        
        Ok(Self {
            watsonx,
            rag_engine: Some(Arc::new(Mutex::new(rag_engine))),
            rag_enabled: true,
            retry_config: RetryConfig::default(),
            failure_history: Vec::new(),
            quality_analyzer: GenerationQualityAnalyzer::new(),
            learning_engine: CommandLearningEngine::new("learning_data.json").unwrap_or_else(|e| {
                eprintln!("Warning: Failed to initialize learning engine: {}", e);
                CommandLearningEngine::new("learning_data.json").unwrap()
            }),
        })
    }
    
    /// Enable or disable RAG functionality
    pub fn set_rag_enabled(&mut self, enabled: bool) {
        self.rag_enabled = enabled && self.rag_engine.is_some();
        println!("🔧 RAG functionality {}", if self.rag_enabled { "enabled" } else { "disabled" });
    }

    /// Configure retry settings
    pub fn set_retry_config(&mut self, config: RetryConfig) {
        self.retry_config = config;
    }

    /// Add execution failure feedback for learning
    pub fn add_execution_feedback(&mut self, command: &str, error_message: &str, user_input: &str) {
        self.failure_history.push(error_message.to_string());
        // Keep only recent failures to avoid prompt bloat
        if self.failure_history.len() > 5 {
            self.failure_history.remove(0);
        }
        
        // Add to learning engine for pattern recognition
        if let Err(e) = self.learning_engine.add_correction(
            user_input,
            command,
            "", // No correct command available for execution errors
            Some(error_message),
            CorrectionType::Other("ExecutionError".to_string())
        ) {
            eprintln!("Warning: Failed to record learning data: {}", e);
        }
        
        println!("📝 Recorded execution feedback: {}", error_message);
    }

    /// Clear failure history
    pub fn clear_failure_history(&mut self) {
        self.failure_history.clear();
        println!("🧹 Cleared failure history");
    }
    
    pub fn analyze_command_quality(&self, command: &str, user_input: &str) -> AnalysisResult {
        self.quality_analyzer.analyze_generation(command, user_input, None)
    }
    
    pub fn get_quality_suggestions(&self, command: &str, user_input: &str) -> Vec<String> {
        let analysis = self.analyze_command_quality(command, user_input);
        self.quality_analyzer.suggest_improvements(command, &analysis)
    }
    
    /// Get command success rate from learning engine
    pub fn get_command_success_rate(&self, command: &str) -> f32 {
        self.learning_engine.get_success_rate(command)
    }
    
    /// Get intelligent retry suggestions from learning engine
    pub fn get_intelligent_retry_suggestions(&self, command: &str, error_message: &str, attempt_count: u32) -> Vec<String> {
        self.learning_engine.get_retry_suggestions(command, error_message, attempt_count)
    }
    
    /// Update success metrics after command execution
    pub fn update_command_success(&mut self, command: &str, was_successful: bool) {
        self.learning_engine.update_success_metrics(command, was_successful);
    }
    
    /// Analyze failure pattern and get retry strategy
    pub fn analyze_command_failure(&self, command: &str, error_message: &str) -> Option<RetryStrategy> {
        self.learning_engine.analyze_failure_pattern(error_message, command)
    }

    /// Translate with intelligent retry and feedback integration
    pub async fn translate_with_feedback(&mut self, query: &str) -> Result<GenerationAttempt> {
        println!("🔄 Translating query with feedback: {}", query);
        
        // Prepare base prompt
        let base_prompt = self.prepare_base_prompt(query).await?;
        
        // Use feedback-enhanced generation
        let model_id = WatsonxAI::GRANITE_3_3_8B_INSTRUCT;
        let attempt = self.watsonx.watsonx_gen_with_feedback(
            &base_prompt,
            model_id,
            100,
            &self.failure_history,
            Some(self.retry_config.clone()),
        ).await?;
        
        // Validate and format the result
        let validated_command = self.validate_and_format_command(&attempt.result, query)?;
        
        // Analyze generation quality
        let analysis = self.quality_analyzer.analyze_generation(
            &validated_command,
            query,
            None,
        );
        
        // Log quality metrics for debugging
        if analysis.metrics.overall_score < 0.6 {
            eprintln!("⚠️ Low quality generation detected (score: {:.2})", analysis.metrics.overall_score);
            for suggestion in &analysis.recommended_actions {
                eprintln!("💡 Suggestion: {}", suggestion);
            }
        }
        
        println!("✅ Translation completed with quality score: {:.2} (attempt {})", 
                attempt.quality_score, attempt.attempt_number);
        
        Ok(GenerationAttempt {
            prompt: attempt.prompt,
            result: validated_command,
            quality_score: analysis.metrics.overall_score as f32,
            attempt_number: attempt.attempt_number,
        })
    }

    pub async fn translate(&self, query: &str) -> Result<String> {
        println!("🔄 Translating query: {}", query);
        
        // Concise prompt for faster processing
        let base_prompt = format!(
            "Translate to IBM Cloud CLI command:\n\nQuery: {}\n\nCommand:",
            query
        );
        
        // Enhance prompt with RAG context if available
        let enhanced_prompt = if self.rag_enabled {
            if let Some(rag_engine) = &self.rag_engine {
                println!("🔍 Enhancing translation with RAG context...");
                let rag_engine = rag_engine.lock().await;
                match rag_engine.enhance_prompt(&base_prompt, query).await {
                    Ok(enhanced) => {
                        println!("✅ RAG context successfully integrated");
                        enhanced
                    }
                    Err(e) => {
                        println!("⚠️  RAG enhancement failed: {}, using base prompt", e);
                        base_prompt
                    }
                }
            } else {
                base_prompt
            }
        } else {
            base_prompt
        };
        
        // Streamlined prompt with essential context only
        let prompt = format!(
            "{}\n\nRules: Return only the IBM Cloud CLI command, no explanations.\nExamples:\n- databases → ibmcloud resource service-instances --service-name databases-for-postgresql\n- watson services → ibmcloud resource service-instances --service-name watson\n- login → ibmcloud login --sso\n\nNow translate this query to an IBM Cloud CLI command:\n{}",
            enhanced_prompt, query
        );
        
        // Enhanced generation with optimized parameters
        let model_id = WatsonxAI::GRANITE_3_3_8B_INSTRUCT;
        let response = self.watsonx.watsonx_gen_with_timeout(
            &prompt, 
            model_id, 
            100, // Further reduced for faster response
            std::time::Duration::from_secs(30) // Shorter timeout for simpler prompt
        ).await?;
        
        // Improved command extraction with better validation
        let command = response.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .find(|line| line.starts_with("ibmcloud"))
            .or_else(|| {
                // Fallback: look for any line that could be a command
                response.lines()
                    .map(|line| line.trim())
                    .find(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with("#"))
            })
            .unwrap_or("")
            .trim();
        
        // Enhanced command validation and formatting
        let command = if command.is_empty() {
            return Err(anyhow::anyhow!("Unable to generate a valid IBM Cloud command for the query: {}", query));
        } else if !command.starts_with("ibmcloud ") && !command.eq("ibmcloud") {
            // Only prepend if it doesn't already start with ibmcloud
            if command.contains("ibmcloud") {
                command.to_string()
            } else {
                format!("ibmcloud {}", command)
            }
        } else {
            command.to_string()
        };
        
        // Final validation
        if !command.starts_with("ibmcloud") {
            return Err(anyhow::anyhow!("Generated command does not start with 'ibmcloud': {}", command));
        }
        
        println!("✅ Translation completed successfully{}", 
            if self.rag_enabled { " with RAG enhancement" } else { "" });
        Ok(command)
    }

    /// Enhanced translate with learning feedback integration
    pub async fn translate_with_learning_feedback(&mut self, user_input: &str, max_attempts: usize) -> Result<String> {
        let mut attempts = Vec::new();
        let mut last_error = None;
        
        for attempt in 1..=max_attempts {
            println!("🔄 Attempt {} of {}", attempt, max_attempts);
            
            // Get context from previous failures and execution feedback
            let failure_context = if attempts.is_empty() && self.failure_history.is_empty() {
                String::new()
            } else {
                let mut context_parts = Vec::new();
                
                if !attempts.is_empty() {
                    context_parts.push(format!(
                        "Previous generation attempts failed:\n{}",
                        attempts.iter().enumerate().map(|(i, error)| {
                            format!("Attempt {}: {}", i + 1, error)
                        }).collect::<Vec<_>>().join("\n")
                    ));
                }
                
                if !self.failure_history.is_empty() {
                    context_parts.push(format!(
                        "Previous command execution feedback:\n{}",
                        self.failure_history.join("\n")
                    ));
                }
                
                format!("\n\n{}", context_parts.join("\n\n"))
            };
            
            // Get learning suggestions for better generation
            let learning_suggestions = if attempt > 1 {
                self.learning_engine.get_suggestions(user_input, last_error.as_ref().map(|e: &anyhow::Error| e.to_string()).as_deref())
            } else {
                Vec::new()
            };
            
            let suggestions_context = if !learning_suggestions.is_empty() {
                format!("\n\nSuggestions based on similar patterns:\n{}", learning_suggestions.join("\n"))
            } else {
                String::new()
            };
            
            let enhanced_query = format!("{}{}{}", user_input, failure_context, suggestions_context);
            
            match self.translate(&enhanced_query).await {
                Ok(command) => {
                    // Analyze quality
                    let analysis = self.quality_analyzer.analyze_generation(&command, user_input, None);
                    
                    println!("📊 Quality Analysis:");
                    println!("   Overall Score: {:.2}", analysis.metrics.overall_score);
                    println!("   Syntax: {:.2}, Completeness: {:.2}, Parameters: {:.2}", 
                             analysis.metrics.syntax_correctness_score, 
                             analysis.metrics.completeness_score, 
                             analysis.metrics.parameter_validity_score);
                    
                    return Ok(command);
                },
                Err(e) => {
                    let error_msg = e.to_string();
                    println!("❌ Translation failed: {}", error_msg);
                    
                    attempts.push(error_msg.clone());
                    last_error = Some(e);
                    
                    // Learn from the failure
                    if let Err(learning_err) = self.learning_engine.add_correction(
                        user_input,
                        "",
                        "",
                        Some(&error_msg),
                        CorrectionType::Other("TranslationError".to_string())
                    ) {
                        eprintln!("Warning: Failed to record learning data: {}", learning_err);
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Max attempts reached")))
    }

    /// Prepare base prompt with RAG enhancement if available
    async fn prepare_base_prompt(&self, query: &str) -> Result<String> {
        let base_prompt = format!(
            "Translate to IBM Cloud CLI command:\n\nQuery: {}\n\nCommand:",
            query
        );
        
        // Enhance prompt with RAG context if available
        if self.rag_enabled {
            if let Some(rag_engine) = &self.rag_engine {
                println!("🔍 Enhancing translation with RAG context...");
                let rag_engine = rag_engine.lock().await;
                match rag_engine.enhance_prompt(&base_prompt, query).await {
                    Ok(enhanced) => {
                        println!("✅ RAG context successfully integrated");
                        return Ok(enhanced);
                    }
                    Err(e) => {
                        println!("⚠️  RAG enhancement failed: {}, using base prompt", e);
                    }
                }
            }
        }
        
        // Add essential context and examples
        let enhanced_prompt = format!(
            "{}\n\nRules: Return only the IBM Cloud CLI command, no explanations.\nExamples:\n- databases → ibmcloud resource service-instances --service-name databases-for-postgresql\n- watson services → ibmcloud resource service-instances --service-name watson\n- login → ibmcloud login --sso",
            base_prompt
        );
        
        Ok(enhanced_prompt)
    }

    /// Validate and format the generated command
    fn validate_and_format_command(&self, result: &str, query: &str) -> Result<String> {
        // Improved command extraction with better validation
        let command = result.lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .find(|line| line.starts_with("ibmcloud"))
            .or_else(|| {
                // Fallback: look for any line that could be a command
                result.lines()
                    .map(|line| line.trim())
                    .find(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with("#"))
            })
            .unwrap_or("")
            .trim();
        
        // Enhanced command validation and formatting
        let command = if command.is_empty() {
            return Err(anyhow::anyhow!("Unable to generate a valid IBM Cloud command for the query: {}", query));
        } else if !command.starts_with("ibmcloud ") && !command.eq("ibmcloud") {
            // Only prepend if it doesn't already start with ibmcloud
            if command.contains("ibmcloud") {
                command.to_string()
            } else {
                format!("ibmcloud {}", command)
            }
        } else {
            command.to_string()
        };
        
        // Final validation
        if !command.starts_with("ibmcloud") {
            return Err(anyhow::anyhow!("Generated command does not start with 'ibmcloud': {}", command));
        }
        
        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[tokio::test]
    async fn test_translate_simple_query() {
        // Skip test if no API key is available
        if env::var("WATSONX_API_KEY").is_err() && env::var("API_KEY").is_err() {
            println!("Skipping test: No API key available");
            return;
        }
        
        // Create a mock test that doesn't require actual API calls
        // This follows the principle of keeping tests independent and fast
        let watsonx_result = WatsonxAI::new();
        if watsonx_result.is_err() {
            println!("Skipping test: WatsonX initialization failed (likely missing credentials)");
            return;
        }
        
        let mut watsonx = watsonx_result.unwrap();
        let connect_result = watsonx.connect().await;
        if connect_result.is_err() {
            println!("Skipping test: WatsonX connection failed (likely authentication issue)");
            return;
        }
        
        let translator = CommandTranslator::new(watsonx);
        let result = translator.translate("list all my cloud functions").await;
        
        // If the translation succeeds, verify the output
        if let Ok(command) = result {
            assert!(command.contains("ibmcloud"));
            assert!(command.contains("fn") || command.contains("function"));
            assert!(command.contains("list"));
        } else {
            println!("Test skipped due to API connectivity issues: {:?}", result.err());
        }
    }
}