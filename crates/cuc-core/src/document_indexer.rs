//! Document indexer trait and types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

/// A document to be indexed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    pub url: Option<String>,
    pub metadata: serde_json::Value,
}

/// Result of an indexing operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingResult {
    pub documents_indexed: usize,
    pub documents_failed: usize,
    pub errors: Vec<String>,
}

/// Configuration for document indexing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub batch_size: usize,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,
            chunk_overlap: 200,
            batch_size: 10,
        }
    }
}

/// Trait for document indexers
///
/// This trait defines the interface for indexing documents into a vector store.
/// It supports web scraping, document chunking, and batch processing.
#[async_trait]
pub trait DocumentIndexer: Send + Sync {
    /// Index a single document
    async fn index_document(&self, document: Document) -> Result<IndexingResult>;

    /// Index multiple documents
    async fn index_documents(&self, documents: Vec<Document>) -> Result<IndexingResult>;

    /// Index documents from a URL
    async fn index_from_url(&self, url: &str) -> Result<IndexingResult>;

    /// Index documents from multiple URLs
    async fn index_from_urls(&self, urls: Vec<String>) -> Result<IndexingResult>;

    /// Index documents from local files
    async fn index_from_file(&self, path: &str) -> Result<IndexingResult>;

    /// Get indexing statistics
    async fn stats(&self) -> Result<serde_json::Value>;
}
