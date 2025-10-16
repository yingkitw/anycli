//! VMware vSphere provider implementation for CUC

use async_trait::async_trait;
use cuc_core::{CloudProvider, CloudProviderType, Result};
use std::process::Command;

/// VMware vSphere provider
pub struct VMwareProvider {
    config: VMwareConfig,
}

/// VMware configuration
#[derive(Debug, Clone)]
pub struct VMwareConfig {
    /// vCenter URL (optional)
    pub vcenter_url: Option<String>,
    /// Username (optional)
    pub username: Option<String>,
}

impl Default for VMwareConfig {
    fn default() -> Self {
        Self {
            vcenter_url: None,
            username: None,
        }
    }
}

impl VMwareProvider {
    /// Create a new VMware provider
    pub fn new() -> Self {
        Self {
            config: VMwareConfig::default(),
        }
    }

    /// Create a new VMware provider with configuration
    pub fn with_config(config: VMwareConfig) -> Self {
        Self { config }
    }
}

impl Default for VMwareProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CloudProvider for VMwareProvider {
    fn provider_type(&self) -> CloudProviderType {
        CloudProviderType::VMware
    }

    async fn is_cli_installed(&self) -> Result<bool> {
        let output = Command::new("which")
            .arg("govc")
            .output();
        
        Ok(output.is_ok() && output.unwrap().status.success())
    }

    async fn is_authenticated(&self) -> Result<bool> {
        let output = Command::new("govc")
            .args(["about"])
            .output();
        
        match output {
            Ok(result) => Ok(result.status.success()),
            Err(_) => Ok(false),
        }
    }

    fn get_rag_context(&self) -> String {
        r#"VMware govc CLI Commands:
- govc about: Display vCenter information
- govc vm.info: Get VM information
- govc vm.power: Control VM power state
- govc host.info: Get ESXi host information
- govc datastore.info: Get datastore information
- govc ls: List inventory objects
- govc find: Search for inventory objects
- govc vm.create: Create a new VM

Common patterns:
- List VMs: govc ls /*/vm
- List hosts: govc ls /*/host
- Get VM info: govc vm.info <vm-name>
- Power on VM: govc vm.power -on <vm-name>
- Power off VM: govc vm.power -off <vm-name>
- List datastores: govc ls /*/datastore
"#.to_string()
    }

    fn validate_command(&self, command: &str) -> Result<()> {
        if !command.starts_with("govc") {
            return Err(anyhow::anyhow!(
                "Invalid VMware command: must start with 'govc'"
            ).into());
        }
        Ok(())
    }

    fn get_command_patterns(&self) -> Vec<String> {
        vec![
            "govc ls /*/vm".to_string(),
            "govc vm.info".to_string(),
            "govc vm.power -on".to_string(),
            "govc host.info".to_string(),
            "govc datastore.info".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_type() {
        let provider = VMwareProvider::new();
        assert_eq!(provider.provider_type(), CloudProviderType::VMware);
    }

    #[test]
    fn test_validate_command() {
        let provider = VMwareProvider::new();
        assert!(provider.validate_command("govc vm.info").is_ok());
        assert!(provider.validate_command("aws s3 ls").is_err());
    }

    #[test]
    fn test_get_rag_context() {
        let provider = VMwareProvider::new();
        let context = provider.get_rag_context();
        assert!(context.contains("govc"));
        assert!(context.contains("vm"));
    }

    #[test]
    fn test_with_config() {
        let config = VMwareConfig {
            vcenter_url: Some("https://vcenter.example.com".to_string()),
            username: Some("admin".to_string()),
        };
        let provider = VMwareProvider::with_config(config.clone());
        assert_eq!(provider.config.vcenter_url, config.vcenter_url);
    }

    #[test]
    fn test_command_patterns() {
        let provider = VMwareProvider::new();
        let patterns = provider.get_command_patterns();
        assert!(patterns.iter().any(|p| p.contains("vm")));
        assert!(patterns.iter().any(|p| p.contains("host")));
    }

    #[test]
    fn test_rag_context_keywords() {
        let provider = VMwareProvider::new();
        let context = provider.get_rag_context();
        assert!(context.contains("ESXi"));
        assert!(context.contains("datastore"));
        assert!(context.contains("vCenter"));
    }
}
