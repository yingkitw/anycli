//! Azure provider implementation for CUC

use async_trait::async_trait;
use cuc_core::{CloudProvider, CloudProviderType, Result};
use std::process::Command;

/// Azure provider
pub struct AzureProvider {
    config: AzureConfig,
}

/// Azure configuration
#[derive(Debug, Clone)]
pub struct AzureConfig {
    /// Azure subscription (optional)
    pub subscription: Option<String>,
    /// Azure resource group (optional)
    pub resource_group: Option<String>,
}

impl Default for AzureConfig {
    fn default() -> Self {
        Self {
            subscription: None,
            resource_group: None,
        }
    }
}

impl AzureProvider {
    /// Create a new Azure provider
    pub fn new() -> Self {
        Self {
            config: AzureConfig::default(),
        }
    }

    /// Create a new Azure provider with configuration
    pub fn with_config(config: AzureConfig) -> Self {
        Self { config }
    }
}

impl Default for AzureProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CloudProvider for AzureProvider {
    fn provider_type(&self) -> CloudProviderType {
        CloudProviderType::Azure
    }

    async fn is_cli_installed(&self) -> Result<bool> {
        let output = Command::new("which")
            .arg("az")
            .output();
        
        Ok(output.is_ok() && output.unwrap().status.success())
    }

    async fn is_authenticated(&self) -> Result<bool> {
        let output = Command::new("az")
            .args(["account", "show"])
            .output();
        
        match output {
            Ok(result) => Ok(result.status.success()),
            Err(_) => Ok(false),
        }
    }

    fn get_rag_context(&self) -> String {
        r#"Azure CLI Commands:
- az login: Authenticate to Azure
- az account: Manage subscriptions
- az vm: Virtual machine management
- az storage: Storage account operations
- az aks: Azure Kubernetes Service
- az functionapp: Azure Functions management
- az group: Resource group management
- az network: Network operations

Common patterns:
- List VMs: az vm list
- List storage accounts: az storage account list
- List AKS clusters: az aks list
- List resource groups: az group list
- Create VM: az vm create
- Create storage account: az storage account create
"#.to_string()
    }

    fn validate_command(&self, command: &str) -> Result<()> {
        if !command.starts_with("az ") && command != "az" {
            return Err(anyhow::anyhow!(
                "Invalid Azure command: must start with 'az'"
            ).into());
        }
        Ok(())
    }

    fn get_command_patterns(&self) -> Vec<String> {
        vec![
            "az vm list".to_string(),
            "az storage account list".to_string(),
            "az aks list".to_string(),
            "az group list".to_string(),
            "az functionapp list".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_type() {
        let provider = AzureProvider::new();
        assert_eq!(provider.provider_type(), CloudProviderType::Azure);
    }

    #[test]
    fn test_validate_command() {
        let provider = AzureProvider::new();
        assert!(provider.validate_command("az vm list").is_ok());
        assert!(provider.validate_command("aws s3 ls").is_err());
    }

    #[test]
    fn test_get_rag_context() {
        let provider = AzureProvider::new();
        let context = provider.get_rag_context();
        assert!(context.contains("az"));
        assert!(context.contains("vm"));
    }

    #[test]
    fn test_with_config() {
        let config = AzureConfig {
            subscription: Some("sub-123".to_string()),
            resource_group: Some("rg-prod".to_string()),
        };
        let provider = AzureProvider::with_config(config.clone());
        assert_eq!(provider.config.subscription, config.subscription);
    }

    #[test]
    fn test_command_patterns() {
        let provider = AzureProvider::new();
        let patterns = provider.get_command_patterns();
        assert!(patterns.iter().any(|p| p.contains("vm")));
        assert!(patterns.iter().any(|p| p.contains("storage")));
    }

    #[test]
    fn test_validate_az_command() {
        let provider = AzureProvider::new();
        assert!(provider.validate_command("az").is_ok());
        assert!(provider.validate_command("az vm list").is_ok());
    }
}
