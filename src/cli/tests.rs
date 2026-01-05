//! Tests for CLI components

#[cfg(test)]
mod tests {
    use crate::cli::{QualityAnalyzer, CommandLearningEngine};
    use tempfile::NamedTempFile;

    #[test]
    fn test_quality_analyzer_analysis() {
        let analyzer = QualityAnalyzer::new();

        // Test good commands
        let good_commands = vec![
            "ibmcloud resource groups",
            "ibmcloud login --sso",
            "ibmcloud target -r us-south -g default",
            "ibmcloud cf apps",
            "ibmcloud plugin list",
            "ibmcloud resource service-instances --service-name databases-for-postgresql",
        ];

        for command in good_commands {
            let analysis = analyzer.analyze(command);
            assert!(analysis.score > 0.6, "Command '{}' should have score > 0.6, got {}", command, analysis.score);
            assert!(analysis.issues.is_empty() || analysis.issues.len() < 2, 
                "Good command '{}' should have few issues", command);
        }

        // Test bad commands
        let bad_commands = vec![
            ("error: command not found", true),
            ("", true),
            ("ibmcloud", true),
        ];

        for (command, should_have_issues) in bad_commands {
            let analysis = analyzer.analyze(command);
            if should_have_issues {
                assert!(analysis.score < 0.6 || !analysis.issues.is_empty(), 
                    "Bad command '{}' should have low score or issues", command);
            }
        }
    }

    #[test]
    fn test_quality_analyzer_validation() {
        let analyzer = QualityAnalyzer::new();

        let validations = vec![
            ("ibmcloud resource groups", true),
            ("ibmcloud login", true),
            ("error", false),
            ("", false),
        ];

        for (cmd, expected) in validations {
            let result = analyzer.is_valid(cmd);
            assert_eq!(result, expected, "Command '{}' validation should be {}", cmd, expected);
        }
    }

    #[tokio::test]
    async fn test_command_learning_corrections() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let mut engine = CommandLearningEngine::new(path).unwrap();

        // Add some corrections
        engine
            .add_correction(
                "list databases".to_string(),
                "ibmcloud resource service-instances --service-name databases-for-postgresql".to_string(),
                Some("Plugin missing error".to_string()),
            )
            .await
            .unwrap();

        engine
            .add_correction(
                "show clusters".to_string(),
                "ibmcloud ks clusters".to_string(),
                None,
            )
            .await
            .unwrap();

        let all_corrections = engine.get_all_corrections();
        assert_eq!(all_corrections.len(), 2, "Should have 2 corrections");
        
        let first = engine.get_learned_command("list databases");
        assert!(first.is_some(), "Should find learned command for 'list databases'");
        assert_eq!(first.unwrap().correct_command, "ibmcloud resource service-instances --service-name databases-for-postgresql");
        
        let second = engine.get_learned_command("show clusters");
        assert!(second.is_some(), "Should find learned command for 'show clusters'");
        assert_eq!(second.unwrap().correct_command, "ibmcloud ks clusters");
    }

    #[tokio::test]
    async fn test_command_learning_similarity() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();

        let mut engine = CommandLearningEngine::new(path).unwrap();

        engine
            .add_correction(
                "list all databases".to_string(),
                "ibmcloud resource service-instances".to_string(),
                None,
            )
            .await
            .unwrap();

        engine
            .add_correction(
                "show my databases".to_string(),
                "ibmcloud resource service-instances --service-name databases".to_string(),
                None,
            )
            .await
            .unwrap();

        let similar = engine.find_similar("list databases", 0.3);
        
        assert!(!similar.is_empty(), "Should find similar commands");
        assert!(similar.len() >= 1, "Should find at least 1 similar command");
        
        // Verify the similar commands contain expected queries
        let queries: Vec<&str> = similar.iter().map(|c| c.query.as_str()).collect();
        assert!(queries.iter().any(|q| q.contains("databases")), 
            "Similar commands should contain 'databases'");
    }
}
