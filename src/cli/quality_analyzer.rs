//! Quality analyzer for assessing command quality

use regex::Regex;
use crate::core::QualityAnalysis;

/// Quality analyzer for IBM Cloud CLI commands
pub struct QualityAnalyzer {
    command_patterns: Vec<Regex>,
}

impl QualityAnalyzer {
    /// Create a new quality analyzer
    pub fn new() -> Self {
        let patterns = vec![
            r"^ibmcloud\s+",
            r"^ibmcloud\s+(login|target|resource|cf|ks|cr|plugin)",
        ];

        let command_patterns = patterns
            .into_iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self { command_patterns }
    }

    /// Analyze the quality of a generated command
    pub fn analyze(&self, command: &str) -> QualityAnalysis {
        let mut score = 0.0;
        let mut max_score = 0.0;
        let mut issues = Vec::new();
        let mut suggestions = Vec::new();

        // Check if starts with ibmcloud
        max_score += 0.3;
        if command.trim().starts_with("ibmcloud") {
            score += 0.3;
        } else {
            issues.push("Command does not start with 'ibmcloud'".to_string());
            suggestions.push("Ensure the command starts with 'ibmcloud'".to_string());
        }

        // Check reasonable length
        max_score += 0.2;
        let trimmed = command.trim();
        if !trimmed.is_empty() && trimmed.len() > 8 && trimmed.len() < 300 {
            score += 0.2;
        } else if trimmed.is_empty() {
            issues.push("Command is empty".to_string());
        } else if trimmed.len() <= 8 {
            issues.push("Command is too short".to_string());
        } else {
            issues.push("Command is too long".to_string());
            suggestions.push("Consider breaking down into multiple commands".to_string());
        }

        // Check for common patterns
        max_score += 0.2;
        let common_patterns = ["resource", "service", "target", "login", "plugin", "cf", "ks", "cr"];
        if common_patterns.iter().any(|pattern| command.contains(pattern)) {
            score += 0.2;
        } else {
            suggestions.push("Command may not follow common IBM Cloud CLI patterns".to_string());
        }

        // Check for error indicators
        max_score += 0.15;
        let error_indicators = ["error", "failed", "invalid", "unknown", "not found"];
        if !error_indicators.iter().any(|indicator| command.to_lowercase().contains(indicator)) {
            score += 0.15;
        } else {
            issues.push("Command contains error indicators".to_string());
        }

        // Check single line
        max_score += 0.15;
        let line_count = command.lines().filter(|line| !line.trim().is_empty()).count();
        if line_count == 1 {
            score += 0.15;
        } else {
            issues.push("Command spans multiple lines".to_string());
            suggestions.push("Use a single-line command".to_string());
        }

        let final_score = if max_score > 0.0 {
            score / max_score
        } else {
            0.0
        };

        QualityAnalysis {
            score: final_score,
            issues,
            suggestions,
        }
    }

    /// Check if a command is likely valid
    pub fn is_valid(&self, command: &str) -> bool {
        let analysis = self.analyze(command);
        analysis.score >= 0.6
    }
}

impl Default for QualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_analysis() {
        let analyzer = QualityAnalyzer::new();

        let good_command = "ibmcloud resource groups";
        let analysis = analyzer.analyze(good_command);
        assert!(analysis.score > 0.6);

        let bad_command = "error: invalid";
        let analysis = analyzer.analyze(bad_command);
        assert!(analysis.score < 0.6);
    }

    #[test]
    fn test_is_valid() {
        let analyzer = QualityAnalyzer::new();

        assert!(analyzer.is_valid("ibmcloud resource groups"));
        assert!(!analyzer.is_valid("error"));
    }
}
