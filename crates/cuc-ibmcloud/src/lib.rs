//! IBM Cloud provider implementation for CUC

use async_trait::async_trait;
use cuc_core::{CloudProvider, CloudProviderType, Result};
use std::process::Command;

/// IBM Cloud provider
pub struct IBMCloudProvider {
    config: IBMCloudConfig,
}

/// IBM Cloud configuration
#[derive(Debug, Clone)]
pub struct IBMCloudConfig {
    /// API endpoint (optional)
    pub api_endpoint: Option<String>,
    /// Region (optional)
    pub region: Option<String>,
}

impl Default for IBMCloudConfig {
    fn default() -> Self {
        Self {
            api_endpoint: None,
            region: None,
        }
    }
}

impl IBMCloudProvider {
    /// Create a new IBM Cloud provider
    pub fn new() -> Self {
        Self {
            config: IBMCloudConfig::default(),
        }
    }

    /// Create a new IBM Cloud provider with configuration
    pub fn with_config(config: IBMCloudConfig) -> Self {
        Self { config }
    }
}

impl Default for IBMCloudProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CloudProvider for IBMCloudProvider {
    fn provider_type(&self) -> CloudProviderType {
        CloudProviderType::IBMCloud
    }

    async fn is_cli_installed(&self) -> Result<bool> {
        let output = Command::new("which")
            .arg("ibmcloud")
            .output();
        
        Ok(output.is_ok() && output.unwrap().status.success())
    }

    async fn is_authenticated(&self) -> Result<bool> {
        let output = Command::new("ibmcloud")
            .args(["target"])
            .output();
        
        match output {
            Ok(result) => Ok(result.status.success()),
            Err(_) => Ok(false),
        }
    }

    fn get_rag_context(&self) -> String {
        r#"IBM Cloud CLI Commands:
- ibmcloud login: Authenticate to IBM Cloud
- ibmcloud target: Set or view target account, region, resource group
- ibmcloud resource: Manage resources (service-instances, groups, etc.)
- ibmcloud ks: Kubernetes Service commands
- ibmcloud ce: Code Engine commands
- ibmcloud cf: Cloud Foundry commands
- ibmcloud iam: Identity and Access Management commands
- ibmcloud plugin: Manage CLI plugins

Common patterns:
- List resources: ibmcloud resource service-instances
- Target resource group: ibmcloud target -g <group>
- List clusters: ibmcloud ks clusters
- Code Engine apps: ibmcloud ce application list
"#.to_string()
    }

    fn validate_command(&self, command: &str) -> Result<()> {
        if !command.starts_with("ibmcloud") {
            return Err(anyhow::anyhow!(
                "Invalid IBM Cloud command: must start with 'ibmcloud'"
            ).into());
        }
        Ok(())
    }

    fn get_command_patterns(&self) -> Vec<String> {
        vec![
            "ibmcloud login".to_string(),
            "ibmcloud target".to_string(),
            "ibmcloud resource service-instances".to_string(),
            "ibmcloud ks clusters".to_string(),
            "ibmcloud ce application list".to_string(),
            "ibmcloud cf apps".to_string(),
            "ibmcloud iam users".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_type() {
        let provider = IBMCloudProvider::new();
        assert_eq!(provider.provider_type(), CloudProviderType::IBMCloud);
    }

    #[test]
    fn test_validate_command() {
        let provider = IBMCloudProvider::new();
        assert!(provider.validate_command("ibmcloud login").is_ok());
        assert!(provider.validate_command("aws s3 ls").is_err());
    }

    #[test]
    fn test_get_rag_context() {
        let provider = IBMCloudProvider::new();
        let context = provider.get_rag_context();
        assert!(context.contains("ibmcloud"));
        assert!(context.contains("resource"));
    }

    #[test]
    fn test_default_provider() {
        let provider = IBMCloudProvider::default();
        assert_eq!(provider.provider_type(), CloudProviderType::IBMCloud);
    }

    #[test]
    fn test_with_config() {
        let config = IBMCloudConfig {
            api_endpoint: Some("https://cloud.ibm.com".to_string()),
            region: Some("us-south".to_string()),
        };
        let provider = IBMCloudProvider::with_config(config.clone());
        assert_eq!(provider.config.region, config.region);
    }

    #[test]
    fn test_command_patterns() {
        let provider = IBMCloudProvider::new();
        let patterns = provider.get_command_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.contains("ibmcloud")));
    }

    #[test]
    fn test_validate_invalid_command() {
        let provider = IBMCloudProvider::new();
        assert!(provider.validate_command("aws s3 ls").is_err());
        assert!(provider.validate_command("gcloud compute instances list").is_err());
    }

    #[test]
    fn test_rag_context_contains_keywords() {
        let provider = IBMCloudProvider::new();
        let context = provider.get_rag_context();
        assert!(context.contains("login"));
        assert!(context.contains("target"));
        assert!(context.contains("Kubernetes") || context.contains("ks"));
        assert!(context.contains("Code Engine"));
    }
}
