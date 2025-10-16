//! RAG (Retrieval-Augmented Generation) engine trait and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Result, VectorDocument};

/// Query for RAG retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGQuery {
    pub query: String,
    pub top_k: usize,
    pub score_threshold: Option<f32>,
    pub filters: Option<Vec<(String, String)>>,
}

impl Default for RAGQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            top_k: 5,
            score_threshold: Some(0.7),
            filters: None,
        }
    }
}

/// Result from RAG retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGResult {
    pub documents: Vec<VectorDocument>,
    pub context: String,
    pub metadata: Option<serde_json::Value>,
}

/// Trait for RAG engines
///
/// This trait defines the interface for Retrieval-Augmented Generation systems.
/// It supports document retrieval, context building, and integration with LLM providers.
#[async_trait]
pub trait RAGEngine: Send + Sync {
    /// Initialize the RAG engine
    async fn initialize(&mut self) -> Result<()>;

    /// Retrieve relevant documents for a query
    async fn retrieve(&self, query: &RAGQuery) -> Result<RAGResult>;

    /// Build context from retrieved documents
    fn build_context(&self, documents: &[VectorDocument]) -> String;

    /// Enhance a prompt with RAG context
    async fn enhance_prompt(&self, prompt: &str, query: &RAGQuery) -> Result<String>;

    /// Get statistics about the RAG engine
    async fn stats(&self) -> Result<serde_json::Value>;

    /// Check if the RAG engine is ready
    fn is_ready(&self) -> bool;
}
