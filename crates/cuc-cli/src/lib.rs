//! CLI interface for IBM Cloud CLI AI

mod translator;
mod command_learning;
mod quality_analyzer;
mod ui;

#[cfg(test)]
mod tests;

pub use translator::CommandTranslator;
pub use command_learning::{CommandLearningEngine, CorrectionType};
pub use quality_analyzer::QualityAnalyzer;
pub use ui::{display_banner, handle_input_with_history};

// Re-export core types
pub use cuc_core::{Error, Result};
