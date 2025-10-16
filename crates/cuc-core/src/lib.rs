//! Core traits and types for CUC (Cloud Universal CLI)
//!
//! This crate defines the fundamental traits and types used across the CUC system.
//! It provides capability-facing interfaces for LLM providers, RAG engines, vector stores,
//! document indexers, and cloud providers, making the system test-friendly and extensible.

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
