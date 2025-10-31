//! Core traits and types for CUC (Cloud Universal CLI)

pub mod llm;
pub mod rag;
pub mod vector_store;
pub mod document_indexer;
pub mod cloud_provider;
pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use llm::{LLMProvider, GenerationConfig, GenerationResult};
pub use rag::{RAGEngine, RAGQuery, RAGResult};
pub use vector_store::{VectorStore, VectorDocument, SearchResult, SearchConfig};
pub use document_indexer::{DocumentIndexer, Document, IndexingResult, IndexingConfig};
pub use cloud_provider::{
    CloudProvider, CloudProviderType, CloudProviderConfig,
    ProviderDetectionResult, detect_provider_from_query,
};
pub use types::*;

