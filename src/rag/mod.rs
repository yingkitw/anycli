//! RAG (Retrieval-Augmented Generation) engine for CUC

mod vector_store;
mod document_indexer;
mod engine;

#[cfg(test)]
mod tests;

pub use vector_store::{LocalVectorStore, QdrantVectorStore};
pub use document_indexer::{LocalDocumentIndexer, WebDocumentIndexer};
pub use engine::LocalRAGEngine;

// Re-export core types for convenience
pub use crate::core::{
    RAGEngine, RAGQuery, RAGResult,
    VectorStore, VectorDocument, SearchResult, SearchConfig,
    DocumentIndexer, Document, IndexingResult, IndexingConfig,
    Error, Result,
};

