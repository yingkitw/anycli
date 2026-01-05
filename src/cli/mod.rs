//! CLI interface for CUC

mod translator;
mod command_learning;
mod quality_analyzer;
mod ui;
mod intent_detector;

#[cfg(test)]
mod tests;

pub use translator::CommandTranslator;
pub use command_learning::{CommandLearningEngine, CorrectionType};
pub use quality_analyzer::QualityAnalyzer;
pub use intent_detector::{IntentDetector, QueryIntent};
pub use ui::{
    display_banner, handle_input_with_history, print_help,
    confirm_execution, execute_command, execute_command_with_provider,
    handle_learning, CommandResult,
};

// Re-export core types
pub use crate::core::{Error, Result};

