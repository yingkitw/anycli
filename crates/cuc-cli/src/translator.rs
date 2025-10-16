//! Command translator for converting natural language to IBM Cloud CLI commands

use cuc_core::{LLMProvider, GenerationConfig, RAGEngine, RAGQuery, Result};

/// Command translator that uses LLM and RAG to translate natural language to CLI commands
pub struct CommandTranslator<L: LLMProvider, R: RAGEngine> {
    llm: L,
    rag: Option<R>,
}

impl<L: LLMProvider, R: RAGEngine> CommandTranslator<L, R> {
    /// Create a new command translator
    pub fn new(llm: L) -> Self {
        Self { llm, rag: None }
    }

    /// Create with RAG support
    pub fn with_rag(llm: L, rag: R) -> Self {
        Self {
            llm,
            rag: Some(rag),
        }
    }

    /// Translate a natural language query to an IBM Cloud CLI command
    pub async fn translate(&self, query: &str) -> Result<String> {
        let prompt = self.build_prompt(query).await?;

        let config = GenerationConfig {
            model_id: self.llm.model_id().to_string(),
            max_tokens: 200,
            ..Default::default()
        };

        let result = self.llm.generate_with_config(&prompt, &config).await?;
        Ok(result.text)
    }

    /// Build the prompt with optional RAG context
    async fn build_prompt(&self, query: &str) -> Result<String> {
        let base_prompt = format!(
            "You are an IBM Cloud CLI expert. Translate the following natural language query into a valid IBM Cloud CLI command.\n\
            Only output the command itself, nothing else.\n\
            \n\
            Query: {}\n\
            Command:",
            query
        );

        if let Some(ref rag) = self.rag {
            if rag.is_ready() {
                let rag_query = RAGQuery {
                    query: query.to_string(),
                    top_k: 3,
                    score_threshold: Some(0.5),
                    filters: None,
                };

                return rag.enhance_prompt(&base_prompt, &rag_query).await;
            }
        }

        Ok(base_prompt)
    }

    /// Check if RAG is available
    pub fn has_rag(&self) -> bool {
        self.rag.as_ref().map_or(false, |r| r.is_ready())
    }

    /// Suggest recovery steps for a failed command
    /// 
    /// # Arguments
    /// * `original_query` - The user's original natural language query
    /// * `failed_command` - The command that failed
    /// * `error_message` - The error message from the failed command
    /// 
    /// # Returns
    /// A suggested next step or corrected command
    pub async fn suggest_recovery(
        &self,
        original_query: &str,
        failed_command: &str,
        error_message: &str,
    ) -> Result<String> {
        let prompt = format!(
            "You are an expert in cloud CLI troubleshooting. A user tried to execute a command but it failed.\n\
            \n\
            Original Intent: {}\n\
            Failed Command: {}\n\
            Error Message: {}\n\
            \n\
            Analyze the error and provide:\n\
            1. A brief explanation of what went wrong (1-2 sentences)\n\
            2. The corrected command or next step to fix the issue\n\
            \n\
            Format your response as:\n\
            Explanation: [your explanation]\n\
            Suggested Command: [the corrected command or next step]\n\
            \n\
            Be concise and practical.",
            original_query,
            failed_command,
            error_message
        );

        let config = GenerationConfig {
            model_id: self.llm.model_id().to_string(),
            max_tokens: 300,
            ..Default::default()
        };

        let result = self.llm.generate_with_config(&prompt, &config).await?;
        Ok(result.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementations for testing would go here
}
