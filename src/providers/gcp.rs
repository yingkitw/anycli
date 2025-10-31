//! GCP provider implementation for CUC

use async_trait::async_trait;
use crate::core::{CloudProvider, CloudProviderType, Result};
use std::process::Command;

/// GCP provider
pub struct GCPProvider {
    config: GCPConfig,
}

/// GCP configuration
#[derive(Debug, Clone)]
pub struct GCPConfig {
    /// GCP project (optional)
    pub project: Option<String>,
    /// GCP region (optional)
    pub region: Option<String>,
}

impl Default for GCPConfig {
    fn default() -> Self {
        Self {
            project: None,
            region: None,
        }
    }
}

impl GCPProvider {
    /// Create a new GCP provider
    pub fn new() -> Self {
        Self {
            config: GCPConfig::default(),
        }
    }

    /// Create a new GCP provider with configuration
    pub fn with_config(config: GCPConfig) -> Self {
        Self { config }
    }
}

impl Default for GCPProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CloudProvider for GCPProvider {
    fn provider_type(&self) -> CloudProviderType {
        CloudProviderType::GCP
    }

    async fn is_cli_installed(&self) -> Result<bool> {
        let output = Command::new("which")
            .arg("gcloud")
            .output();
        
        Ok(output.is_ok() && output.unwrap().status.success())
    }

    async fn is_authenticated(&self) -> Result<bool> {
        let output = Command::new("gcloud")
            .args(["auth", "list"])
            .output();
        
        match output {
            Ok(result) => Ok(result.status.success()),
            Err(_) => Ok(false),
        }
    }

    fn get_rag_context(&self) -> String {
        r#"GCP gcloud CLI Commands:
- gcloud auth login: Authenticate to GCP
- gcloud config set project: Set active project
- gcloud compute: Compute Engine operations
- gcloud storage: Cloud Storage operations
- gcloud container: GKE cluster management
- gcloud functions: Cloud Functions management
- gcloud iam: Identity and Access Management
- gcloud sql: Cloud SQL operations

Common patterns:
- List compute instances: gcloud compute instances list
- List storage buckets: gcloud storage buckets list
- List GKE clusters: gcloud container clusters list
- List Cloud Functions: gcloud functions list
- Create compute instance: gcloud compute instances create
"#.to_string()
    }

    fn validate_command(&self, command: &str) -> Result<()> {
        if !command.starts_with("gcloud") {
            return Err(anyhow::anyhow!(
                "Invalid GCP command: must start with 'gcloud'"
            ).into());
        }
        Ok(())
    }

    fn get_command_patterns(&self) -> Vec<String> {
        vec![
            "gcloud compute instances list".to_string(),
            "gcloud storage buckets list".to_string(),
            "gcloud container clusters list".to_string(),
            "gcloud functions list".to_string(),
            "gcloud iam service-accounts list".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_type() {
        let provider = GCPProvider::new();
        assert_eq!(provider.provider_type(), CloudProviderType::GCP);
    }

    #[test]
    fn test_validate_command() {
        let provider = GCPProvider::new();
        assert!(provider.validate_command("gcloud compute instances list").is_ok());
        assert!(provider.validate_command("az vm list").is_err());
    }

    #[test]
    fn test_get_rag_context() {
        let provider = GCPProvider::new();
        let context = provider.get_rag_context();
        assert!(context.contains("gcloud"));
        assert!(context.contains("compute"));
    }

    #[test]
    fn test_with_config() {
        let config = GCPConfig {
            project: Some("my-project".to_string()),
            region: Some("us-central1".to_string()),
        };
        let provider = GCPProvider::with_config(config.clone());
        assert_eq!(provider.config.project, config.project);
    }

    #[test]
    fn test_command_patterns() {
        let provider = GCPProvider::new();
        let patterns = provider.get_command_patterns();
        assert!(patterns.iter().any(|p| p.contains("compute")));
        assert!(patterns.iter().any(|p| p.contains("storage")));
    }
}
