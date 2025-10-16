//! WatsonX AI integration for IBM Cloud CLI AI
//!
//! This crate provides the WatsonX implementation of the LLMProvider trait.

mod client;
mod config;

#[cfg(test)]
mod tests;

pub use client::WatsonxClient;
pub use config::WatsonxConfig;

// Re-export core types for convenience
pub use cuc_core::{
    LLMProvider, GenerationConfig, GenerationResult, GenerationAttempt,
    RetryConfig, Error, Result,
};
