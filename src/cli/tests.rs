//! Snapshot tests for CLI components

#[cfg(test)]
mod snapshot_tests {
    use crate::cli::{QualityAnalyzer, CommandLearningEngine};
    use insta::assert_yaml_snapshot;
    use tempfile::NamedTempFile;

    #[test]
    fn test_quality_analyzer_snapshots() {
        let analyzer = QualityAnalyzer::new();

        let test_commands = vec![
            ("ibmcloud resource groups", "good_resource_command"),
            ("ibmcloud login --sso", "login_sso_command"),
            ("ibmcloud target -r us-south -g default", "target_command"),
            ("ibmcloud cf apps", "cf_apps_command"),
            ("ibmcloud plugin list", "plugin_list_command"),
            ("error: command not found", "error_command"),
            ("", "empty_command"),
            ("ibmcloud", "incomplete_command"),
            ("ibmcloud resource service-instances --service-name databases-for-postgresql", "long_command"),
        ];

        for (command, name) in test_commands {
            let analysis = analyzer.analyze(command);
            assert_yaml_snapshot!(format!("quality_analysis_{}", name), analysis);
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

        let results: Vec<_> = validations
            .iter()
            .map(|(cmd, _expected)| (cmd, analyzer.is_valid(cmd)))
            .collect();

        assert_yaml_snapshot!("quality_validations", results);
    }

    #[tokio::test]
    #[ignore]
    async fn test_command_learning_snapshot() {
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
        assert_yaml_snapshot!("command_corrections", {
            "[].timestamp" => "[timestamp]",
        }, all_corrections);
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
        
        assert_yaml_snapshot!("similar_commands", {
            "[].timestamp" => "[timestamp]",
        }, similar);
    }
}
