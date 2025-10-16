//! Vector store trait and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

/// A document stored in the vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    pub id: String,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: serde_json::Value,
    pub score: Option<f32>,
}

/// Search result from vector store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub documents: Vec<VectorDocument>,
    pub total: usize,
}

/// Configuration for vector search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    pub top_k: usize,
    pub score_threshold: Option<f32>,
    pub filters: Option<serde_json::Value>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            top_k: 5,
            score_threshold: Some(0.7),
            filters: None,
        }
    }
}

/// Trait for vector stores (e.g., Qdrant, Pinecone, etc.)
///
/// This trait defines the interface for vector database operations.
/// It supports document storage, retrieval, and similarity search.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Initialize the vector store connection
    async fn connect(&mut self) -> Result<()>;

    /// Store a document in the vector store
    async fn store(&self, document: VectorDocument) -> Result<String>;

    /// Store multiple documents in batch
    async fn store_batch(&self, documents: Vec<VectorDocument>) -> Result<Vec<String>>;

    /// Search for similar documents
    async fn search(&self, query: &str, config: &SearchConfig) -> Result<SearchResult>;

    /// Search using a vector embedding
    async fn search_by_vector(&self, vector: Vec<f32>, config: &SearchConfig) -> Result<SearchResult>;

    /// Get a document by ID
    async fn get(&self, id: &str) -> Result<Option<VectorDocument>>;

    /// Delete a document by ID
    async fn delete(&self, id: &str) -> Result<bool>;

    /// Clear all documents from the store
    async fn clear(&self) -> Result<()>;

    /// Get the total number of documents
    async fn count(&self) -> Result<usize>;

    /// Check if the vector store is connected
    fn is_connected(&self) -> bool;
}
