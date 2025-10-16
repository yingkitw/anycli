//! Error types for IBM Cloud CLI AI

use thiserror::Error;

/// Result type alias using our custom Error type
pub type Result<T> = std::result::Result<T, Error>;

/// Core error types for the IBM Cloud CLI AI system
#[derive(Error, Debug)]
pub enum Error {
    #[error("LLM provider error: {0}")]
    LLMProvider(String),

    #[error("RAG engine error: {0}")]
    RAGEngine(String),

    #[error("Vector store error: {0}")]
    VectorStore(String),

    #[error("Document indexer error: {0}")]
    DocumentIndexer(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Error::Other(err.to_string())
    }
}
