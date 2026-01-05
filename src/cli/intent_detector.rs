//! Intent detection for natural language queries

use regex::Regex;

/// Detected intent from a natural language query
#[derive(Debug, Clone, PartialEq)]
pub enum QueryIntent {
    /// Regular command translation
    CommandTranslation,
    /// Deploy to Code Engine
    DeployToCodeEngine {
        app_name: Option<String>,
        project_name: Option<String>,
    },
    /// Unknown intent
    Unknown,
}

/// Intent detector for natural language queries
pub struct IntentDetector {
    deploy_patterns: Vec<Regex>,
}

impl IntentDetector {
    pub fn new() -> Self {
        let deploy_patterns = vec![
            r"(?i)\bdeploy\b.*\b(code\s*engine|codeengine|ce)\b",
            r"(?i)\bdeploy\b.*\b(app|application)\b",
            r"(?i)\bdeploy\b.*\bto\b.*\b(cloud|ibm)\b",
            r"(?i)\bpublish\b.*\b(app|application)\b",
            r"(?i)\bpush\b.*\b(code\s*engine|codeengine)\b",
        ];

        let deploy_patterns = deploy_patterns
            .into_iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect();

        Self { deploy_patterns }
    }

    /// Detect intent from a natural language query
    pub fn detect(&self, query: &str) -> QueryIntent {
        let query_lower = query.to_lowercase();

        // Check for deployment intent
        for pattern in &self.deploy_patterns {
            if pattern.is_match(&query_lower) {
                // Try to extract app name and project name
                let app_name = self.extract_app_name(query);
                let project_name = self.extract_project_name(query);
                
                return QueryIntent::DeployToCodeEngine {
                    app_name,
                    project_name,
                };
            }
        }

        QueryIntent::CommandTranslation
    }

    /// Extract app name from query
    fn extract_app_name(&self, query: &str) -> Option<String> {
        // Look for patterns like "deploy myapp" or "deploy app named myapp"
        let patterns = vec![
            r"(?i)deploy\s+(?:app\s+)?(?:named\s+)?([a-z0-9-]+)",
            r"(?i)deploy\s+(?:the\s+)?([a-z0-9-]+)\s+(?:app|application)",
            r"(?i)app\s+(?:named\s+)?([a-z0-9-]+)",
        ];

        let stop_words = ["app", "application", "to", "the", "my", "this", "code", "engine", "ce"];

        for pattern_str in patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                if let Some(captures) = pattern.captures(query) {
                    if let Some(name) = captures.get(1) {
                        let name = name.as_str().to_string();
                        // Filter out common words and ensure it's a valid identifier
                        if !stop_words.contains(&name.as_str()) && name.len() > 2 {
                            return Some(name);
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract project name from query
    fn extract_project_name(&self, query: &str) -> Option<String> {
        // Look for patterns like "to project myproject" or "in project myproject"
        let patterns = vec![
            r"(?i)(?:to|in)\s+project\s+(\w+)",
            r"(?i)project\s+(?:named\s+)?(\w+)",
        ];

        for pattern_str in patterns {
            if let Ok(pattern) = Regex::new(pattern_str) {
                if let Some(captures) = pattern.captures(query) {
                    if let Some(name) = captures.get(1) {
                        return Some(name.as_str().to_string());
                    }
                }
            }
        }
        None
    }
}

impl Default for IntentDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_intent_detection() {
        let detector = IntentDetector::new();

        assert_eq!(
            detector.detect("deploy to code engine"),
            QueryIntent::DeployToCodeEngine {
                app_name: None,
                project_name: None,
            }
        );

        assert_eq!(
            detector.detect("deploy my app to code engine"),
            QueryIntent::DeployToCodeEngine {
                app_name: None, // "my" is filtered out as a stop word
                project_name: None,
            }
        );

        assert_eq!(
            detector.detect("deploy myapp to code engine"),
            QueryIntent::DeployToCodeEngine {
                app_name: Some("myapp".to_string()),
                project_name: None,
            }
        );

        assert_eq!(
            detector.detect("deploy app named myapp to project myproject"),
            QueryIntent::DeployToCodeEngine {
                app_name: Some("myapp".to_string()),
                project_name: Some("myproject".to_string()),
            }
        );

        assert_eq!(
            detector.detect("list my resources"),
            QueryIntent::CommandTranslation
        );
    }
}

