use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub overall_score: f64,
    pub command_structure_score: f64,
    pub parameter_validity_score: f64,
    pub context_relevance_score: f64,
    pub syntax_correctness_score: f64,
    pub completeness_score: f64,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub metrics: QualityMetrics,
    pub confidence_level: f64,
    pub improvement_areas: Vec<String>,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GenerationQualityAnalyzer {
    ibm_cloud_commands: HashMap<String, Vec<String>>,
    common_parameters: HashMap<String, Vec<String>>,
    quality_patterns: Vec<QualityPattern>,
}

#[derive(Debug, Clone)]
struct QualityPattern {
    pattern: String,
    weight: f64,
    category: QualityCategory,
}

#[derive(Debug, Clone)]
enum QualityCategory {
    Structure,
    Parameters,
    Context,
    Syntax,
    Completeness,
}

impl GenerationQualityAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            ibm_cloud_commands: HashMap::new(),
            common_parameters: HashMap::new(),
            quality_patterns: Vec::new(),
        };
        
        analyzer.initialize_command_knowledge();
        analyzer.initialize_quality_patterns();
        analyzer
    }
    
    fn initialize_command_knowledge(&mut self) {
        // Initialize IBM Cloud CLI command knowledge base
        let commands = vec![
            ("ibmcloud", vec!["login", "target", "logout", "config", "update"]),
            ("ibmcloud account", vec!["list", "show", "orgs", "spaces"]),
            ("ibmcloud resource", vec!["groups", "service-instances", "service-keys"]),
            ("ibmcloud cf", vec!["apps", "services", "routes", "domains"]),
            ("ibmcloud ks", vec!["clusters", "workers", "worker-pools"]),
            ("ibmcloud cr", vec!["images", "namespaces", "tokens"]),
            ("ibmcloud watson", vec!["services", "credentials", "models"]),
            ("ibmcloud code-engine", vec!["projects", "applications", "jobs"]),
        ];
        
        for (base_cmd, subcommands) in commands {
            self.ibm_cloud_commands.insert(base_cmd.to_string(), subcommands.iter().map(|s| s.to_string()).collect());
        }
        
        // Initialize common parameters
        let parameters = vec![
            ("global", vec!["--help", "-h", "--version", "-v", "--output", "-o", "--quiet", "-q"]),
            ("target", vec!["--resource-group", "-g", "--cf-org", "-o", "--cf-space", "-s"]),
            ("list", vec!["--output", "-o", "--resource-group", "-g"]),
            ("create", vec!["--name", "-n", "--resource-group", "-g"]),
            ("delete", vec!["--force", "-f"]),
        ];
        
        for (context, params) in parameters {
            self.common_parameters.insert(context.to_string(), params.iter().map(|s| s.to_string()).collect());
        }
    }
    
    fn initialize_quality_patterns(&mut self) {
        self.quality_patterns = vec![
            // Structure patterns
            QualityPattern {
                pattern: r"^ibmcloud\s+".to_string(),
                weight: 0.3,
                category: QualityCategory::Structure,
            },
            QualityPattern {
                pattern: r"\s+--\w+".to_string(),
                weight: 0.2,
                category: QualityCategory::Parameters,
            },
            // Syntax patterns
            QualityPattern {
                pattern: r"^[^\s]+(?:\s+[^\s]+)*$".to_string(),
                weight: 0.15,
                category: QualityCategory::Syntax,
            },
            // Completeness patterns
            QualityPattern {
                pattern: r"(?:list|show|get|create|delete|update)".to_string(),
                weight: 0.2,
                category: QualityCategory::Completeness,
            },
        ];
    }
    
    pub fn analyze_generation(&self, command: &str, user_input: &str, context: Option<&str>) -> AnalysisResult {
        let metrics = self.calculate_quality_metrics(command, user_input, context);
        let confidence_level = self.calculate_confidence_level(&metrics);
        let improvement_areas = self.identify_improvement_areas(&metrics);
        let recommended_actions = self.generate_recommendations(command, user_input, &metrics);
        
        AnalysisResult {
            metrics,
            confidence_level,
            improvement_areas,
            recommended_actions,
        }
    }
    
    fn calculate_quality_metrics(&self, command: &str, user_input: &str, context: Option<&str>) -> QualityMetrics {
        let command_structure_score = self.assess_command_structure(command);
        let parameter_validity_score = self.assess_parameter_validity(command);
        let context_relevance_score = self.assess_context_relevance(command, user_input, context);
        let syntax_correctness_score = self.assess_syntax_correctness(command);
        let completeness_score = self.assess_completeness(command, user_input);
        
        let overall_score = (
            command_structure_score * 0.25 +
            parameter_validity_score * 0.2 +
            context_relevance_score * 0.2 +
            syntax_correctness_score * 0.15 +
            completeness_score * 0.2
        ).min(1.0).max(0.0);
        
        let suggestions = self.generate_quality_suggestions(command, &[
            ("structure", command_structure_score),
            ("parameters", parameter_validity_score),
            ("context", context_relevance_score),
            ("syntax", syntax_correctness_score),
            ("completeness", completeness_score),
        ]);
        
        QualityMetrics {
            overall_score,
            command_structure_score,
            parameter_validity_score,
            context_relevance_score,
            syntax_correctness_score,
            completeness_score,
            suggestions,
        }
    }
    
    fn assess_command_structure(&self, command: &str) -> f64 {
        let mut score: f64 = 0.0;
        
        // Check if starts with ibmcloud
        if command.trim().starts_with("ibmcloud") {
            score += 0.4;
        }
        
        // Check for valid command hierarchy
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.len() >= 2 {
            let base_command = parts[0..2].join(" ");
            if self.ibm_cloud_commands.contains_key(&base_command) {
                score += 0.3;
                
                // Check for valid subcommand
                if parts.len() >= 3 {
                    if let Some(subcommands) = self.ibm_cloud_commands.get(&base_command) {
                        if subcommands.contains(&parts[2].to_string()) {
                            score += 0.3;
                        }
                    }
                }
            }
        }
        
        score.min(1.0)
    }
    
    fn assess_parameter_validity(&self, command: &str) -> f64 {
        let mut score: f64 = 0.5; // Base score
        let mut parameter_count = 0;
        let mut valid_parameters = 0;
        
        // Extract parameters (--flag or -f format)
        let parameter_regex = regex::Regex::new(r"(--?[a-zA-Z][a-zA-Z0-9-]*)").unwrap();
        
        for cap in parameter_regex.captures_iter(command) {
            parameter_count += 1;
            let param = &cap[1];
            
            // Check against common parameters
            let mut is_valid = false;
            for params in self.common_parameters.values() {
                if params.contains(&param.to_string()) {
                    is_valid = true;
                    break;
                }
            }
            
            if is_valid {
                valid_parameters += 1;
            }
        }
        
        if parameter_count > 0 {
            score = (valid_parameters as f64 / parameter_count as f64) * 0.8 + 0.2;
        }
        
        score.min(1.0)
    }
    
    fn assess_context_relevance(&self, command: &str, user_input: &str, _context: Option<&str>) -> f64 {
        let mut score = 0.0;
        
        // Extract key terms from user input
        let user_input_lower = user_input.to_lowercase();
        let user_terms: Vec<&str> = user_input_lower
            .split_whitespace()
            .filter(|word| word.len() > 2)
            .collect();
        
        let command_lower = command.to_lowercase();
        
        // Check how many user terms appear in the command
        let mut matching_terms = 0;
        for term in &user_terms {
            if command_lower.contains(term) {
                matching_terms += 1;
            }
        }
        
        if !user_terms.is_empty() {
            score = matching_terms as f64 / user_terms.len() as f64;
        }
        
        // Bonus for action words alignment
        let action_words = ["list", "show", "create", "delete", "update", "get", "set"];
        for action in &action_words {
            if user_input.to_lowercase().contains(action) && command_lower.contains(action) {
                score += 0.2;
                break;
            }
        }
        
        score.min(1.0)
    }
    
    fn assess_syntax_correctness(&self, command: &str) -> f64 {
        let mut score: f64 = 1.0;
        
        // Check for basic syntax issues
        if command.trim().is_empty() {
            return 0.0;
        }
        
        // Check for multiple consecutive spaces
        if command.contains("  ") {
            score -= 0.1;
        }
        
        // Check for proper parameter format
        let invalid_param_regex = regex::Regex::new(r"\s-[^-\s]\w+").unwrap();
        if invalid_param_regex.is_match(command) {
            score -= 0.2;
        }
        
        // Check for unmatched quotes
        let quote_count = command.chars().filter(|&c| c == '"' || c == '\'').count();
        if quote_count % 2 != 0 {
            score -= 0.3;
        }
        
        score.max(0.0)
    }
    
    fn assess_completeness(&self, command: &str, user_input: &str) -> f64 {
        let mut score: f64 = 0.5; // Base score
        
        // Check if command has an action verb
        let action_words = ["list", "show", "create", "delete", "update", "get", "set", "login", "logout"];
        let has_action = action_words.iter().any(|&action| command.to_lowercase().contains(action));
        
        if has_action {
            score += 0.3;
        }
        
        // Check if user requested specific output format and command includes it
        if user_input.to_lowercase().contains("json") && command.contains("--output json") {
            score += 0.1;
        }
        
        if user_input.to_lowercase().contains("yaml") && command.contains("--output yaml") {
            score += 0.1;
        }
        
        // Check for resource group specification when needed
        if user_input.to_lowercase().contains("resource group") && command.contains("--resource-group") {
            score += 0.1;
        }
        
        score.min(1.0)
    }
    
    fn calculate_confidence_level(&self, metrics: &QualityMetrics) -> f64 {
        // Confidence is based on consistency across metrics
        let scores = [
            metrics.command_structure_score,
            metrics.parameter_validity_score,
            metrics.context_relevance_score,
            metrics.syntax_correctness_score,
            metrics.completeness_score,
        ];
        
        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance = scores.iter().map(|score| (score - mean).powi(2)).sum::<f64>() / scores.len() as f64;
        let std_dev = variance.sqrt();
        
        // Higher consistency (lower std dev) = higher confidence
        let consistency_factor = (1.0 - std_dev).max(0.0);
        
        // Combine with overall score
        (metrics.overall_score * 0.7 + consistency_factor * 0.3).min(1.0)
    }
    
    fn identify_improvement_areas(&self, metrics: &QualityMetrics) -> Vec<String> {
        let mut areas = Vec::new();
        
        if metrics.command_structure_score < 0.7 {
            areas.push("Command structure and hierarchy".to_string());
        }
        
        if metrics.parameter_validity_score < 0.7 {
            areas.push("Parameter usage and validity".to_string());
        }
        
        if metrics.context_relevance_score < 0.7 {
            areas.push("Relevance to user request".to_string());
        }
        
        if metrics.syntax_correctness_score < 0.7 {
            areas.push("Syntax and formatting".to_string());
        }
        
        if metrics.completeness_score < 0.7 {
            areas.push("Command completeness".to_string());
        }
        
        areas
    }
    
    fn generate_recommendations(&self, command: &str, user_input: &str, metrics: &QualityMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if metrics.command_structure_score < 0.7 {
            recommendations.push("Ensure command starts with 'ibmcloud' and follows proper hierarchy".to_string());
        }
        
        if metrics.parameter_validity_score < 0.7 {
            recommendations.push("Use standard IBM Cloud CLI parameters (--help for reference)".to_string());
        }
        
        if metrics.context_relevance_score < 0.7 {
            recommendations.push("Include more specific terms from the user request".to_string());
        }
        
        if metrics.syntax_correctness_score < 0.7 {
            recommendations.push("Check for proper spacing and parameter formatting".to_string());
        }
        
        if metrics.completeness_score < 0.7 {
            recommendations.push("Add missing action verbs or required parameters".to_string());
        }
        
        // Add specific suggestions based on user input
        if user_input.to_lowercase().contains("list") && !command.contains("list") {
            recommendations.push("Consider using 'list' subcommand for listing resources".to_string());
        }
        
        if user_input.to_lowercase().contains("json") && !command.contains("--output") {
            recommendations.push("Add '--output json' for JSON formatted output".to_string());
        }
        
        recommendations
    }
    
    fn generate_quality_suggestions(&self, _command: &str, scores: &[(&str, f64)]) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        for (category, score) in scores {
            if *score < 0.6 {
                match *category {
                    "structure" => suggestions.push("Improve command structure with proper IBM Cloud CLI hierarchy".to_string()),
                    "parameters" => suggestions.push("Use valid IBM Cloud CLI parameters".to_string()),
                    "context" => suggestions.push("Better align command with user intent".to_string()),
                    "syntax" => suggestions.push("Fix syntax and formatting issues".to_string()),
                    "completeness" => suggestions.push("Add missing components for complete command".to_string()),
                    _ => {}
                }
            }
        }
        
        suggestions
    }
    
    pub fn suggest_improvements(&self, command: &str, analysis: &AnalysisResult) -> Vec<String> {
        let mut improvements = Vec::new();
        
        // Add specific improvement suggestions based on analysis
        improvements.extend(analysis.recommended_actions.clone());
        
        // Add command-specific improvements
        if !command.starts_with("ibmcloud") {
            improvements.push("Prefix command with 'ibmcloud'".to_string());
        }
        
        if analysis.metrics.overall_score < 0.5 {
            improvements.push("Consider completely rephrasing the command".to_string());
        }
        
        improvements
    }
}

impl Default for GenerationQualityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}