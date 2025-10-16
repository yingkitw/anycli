//! Core traits and types for IBM Cloud CLI AI
//!
//! This crate defines the fundamental traits and types used across the IBM Cloud CLI AI system.
//! It provides capability-facing interfaces for LLM providers, RAG engines, vector stores, and
//! document indexers, making the system test-friendly and extensible.

pub mod llm;
pub mod rag;
pub mod vector_store;
pub mod document_indexer;
pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use llm::{LLMProvider, GenerationConfig, GenerationResult};
pub use rag::{RAGEngine, RAGQuery, RAGResult};
pub use vector_store::{VectorStore, VectorDocument, SearchResult, SearchConfig};
pub use document_indexer::{DocumentIndexer, Document, IndexingResult, IndexingConfig};
pub use types::*;
