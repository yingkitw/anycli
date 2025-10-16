//! Snapshot tests for WatsonX client

#[cfg(test)]
mod snapshot_tests {
    use crate::{WatsonxClient, WatsonxConfig, LLMProvider};
    use insta::assert_yaml_snapshot;

    #[test]
    fn test_config_snapshot() {
        let config = WatsonxConfig {
            api_key: "test_api_key_redacted".to_string(),
            project_id: "test_project_id".to_string(),
            iam_url: "iam.cloud.ibm.com".to_string(),
            api_url: "https://us-south.ml.cloud.ibm.com".to_string(),
        };

        assert_yaml_snapshot!(config, @r###"
        ---
        api_key: test_api_key_redacted
        project_id: test_project_id
        iam_url: iam.cloud.ibm.com
        api_url: "https://us-south.ml.cloud.ibm.com"
        "###);
    }

    #[test]
    fn test_quality_assessment_snapshot() {
        let config = WatsonxConfig::new("test_key".to_string(), "test_project".to_string());
        let client = WatsonxClient::new(config).unwrap();

        let test_cases = vec![
            ("ibmcloud resource groups", "good_command"),
            ("ibmcloud login --sso", "login_command"),
            ("ibmcloud target -r us-south", "target_command"),
            ("error: invalid command", "error_command"),
            ("", "empty_command"),
        ];

        for (command, name) in test_cases {
            let score = client.assess_quality(command, "test prompt");
            assert_yaml_snapshot!(format!("quality_{}", name), score);
        }
    }

    #[test]
    fn test_model_constants() {
        assert_yaml_snapshot!("model_granite_4_h_small", WatsonxClient::GRANITE_4_H_SMALL);
        assert_yaml_snapshot!("model_granite_3_3_8b", WatsonxClient::GRANITE_3_3_8B_INSTRUCT);
    }
}
