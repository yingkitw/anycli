//! Command translator for converting natural language to IBM Cloud CLI commands

use crate::core::{LLMProvider, GenerationConfig, RAGEngine, RAGQuery, Result};

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
        // Try to get RAG context for better suggestions
        let mut rag_context = String::new();
        if let Some(ref rag) = self.rag {
            if rag.is_ready() {
                let rag_query = RAGQuery {
                    query: format!("troubleshooting error: {}", error_message),
                    top_k: 2,
                    score_threshold: Some(0.5),
                    filters: None,
                };
                
                if let Ok(rag_result) = rag.retrieve(&rag_query).await {
                    if !rag_result.documents.is_empty() {
                        rag_context = format!("\n\nRELEVANT DOCUMENTATION:\n{}\n", 
                            rag_result.documents.iter()
                                .take(2)
                                .map(|d| format!("- {}", d.content))
                                .collect::<Vec<_>>()
                                .join("\n"));
                    }
                }
            }
        }

        let prompt = format!(
            "You are a cloud CLI expert. A command failed and you must provide the EXACT fix.\n\
            \n\
            USER WANTED: {}\n\
            COMMAND THAT FAILED: {}\n\
            ERROR MESSAGE:\n{}\n\
            {}\
            \n\
            YOUR TASK: Provide a clear, actionable solution.\n\
            \n\
            ANALYZE THE ERROR:\n\
            - If it says \"Not logged in\" → tell user to run: ibmcloud login\n\
            - If it says \"Plugin not found\" → tell user to run: ibmcloud plugin install <plugin-name>\n\
            - If it says \"command not found\" → provide the correct command syntax\n\
            - If it says \"Resource not found\" → suggest how to list available resources\n\
            - If it says \"Permission denied\" → explain what permissions are needed\n\
            \n\
            RESPONSE FORMAT (be specific and direct):\n\
            \n\
            Problem: [One sentence: what went wrong]\n\
            \n\
            Fix: [The exact command(s) to run]\n\
            $ <actual-command-here>\n\
            \n\
            DO NOT:\n\
            - Say \"avoid speculation\"\n\
            - Give generic advice\n\
            - Be vague\n\
            \n\
            DO:\n\
            - Give the EXACT command to run\n\
            - Be specific and actionable\n\
            - Include command examples with $\n\
            \n\
            Example good response:\n\
            Problem: You are not authenticated to IBM Cloud.\n\
            \n\
            Fix: Log in to IBM Cloud first:\n\
            $ ibmcloud login\n\
            \n\
            Then retry your original command.",
            original_query,
            failed_command,
            error_message,
            rag_context
        );

        let config = GenerationConfig {
            model_id: self.llm.model_id().to_string(),
            max_tokens: 400,
            temperature: Some(0.3), // Lower temperature for more focused responses
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
