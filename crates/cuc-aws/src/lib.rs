//! AWS provider implementation for CUC

use async_trait::async_trait;
use cuc_core::{CloudProvider, CloudProviderType, Result};
use std::process::Command;

/// AWS provider
pub struct AWSProvider {
    config: AWSConfig,
}

/// AWS configuration
#[derive(Debug, Clone)]
pub struct AWSConfig {
    /// AWS region (optional)
    pub region: Option<String>,
    /// AWS profile (optional)
    pub profile: Option<String>,
}

impl Default for AWSConfig {
    fn default() -> Self {
        Self {
            region: None,
            profile: None,
        }
    }
}

impl AWSProvider {
    /// Create a new AWS provider
    pub fn new() -> Self {
        Self {
            config: AWSConfig::default(),
        }
    }

    /// Create a new AWS provider with configuration
    pub fn with_config(config: AWSConfig) -> Self {
        Self { config }
    }
}

impl Default for AWSProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CloudProvider for AWSProvider {
    fn provider_type(&self) -> CloudProviderType {
        CloudProviderType::AWS
    }

    async fn is_cli_installed(&self) -> Result<bool> {
        let output = Command::new("which")
            .arg("aws")
            .output();
        
        Ok(output.is_ok() && output.unwrap().status.success())
    }

    async fn is_authenticated(&self) -> Result<bool> {
        let output = Command::new("aws")
            .args(["sts", "get-caller-identity"])
            .output();
        
        match output {
            Ok(result) => Ok(result.status.success()),
            Err(_) => Ok(false),
        }
    }

    fn get_rag_context(&self) -> String {
        r#"AWS CLI Commands:
- aws configure: Configure AWS CLI credentials
- aws sts get-caller-identity: Get current user identity
- aws ec2: EC2 instance management
- aws s3: S3 storage operations
- aws lambda: Lambda function management
- aws eks: Elastic Kubernetes Service
- aws iam: Identity and Access Management
- aws cloudformation: Infrastructure as code

Common patterns:
- List EC2 instances: aws ec2 describe-instances
- List S3 buckets: aws s3 ls
- List Lambda functions: aws lambda list-functions
- List EKS clusters: aws eks list-clusters
- Create S3 bucket: aws s3 mb s3://bucket-name
"#.to_string()
    }

    fn validate_command(&self, command: &str) -> Result<()> {
        if !command.starts_with("aws") {
            return Err(anyhow::anyhow!(
                "Invalid AWS command: must start with 'aws'"
            ).into());
        }
        Ok(())
    }

    fn get_command_patterns(&self) -> Vec<String> {
        vec![
            "aws ec2 describe-instances".to_string(),
            "aws s3 ls".to_string(),
            "aws lambda list-functions".to_string(),
            "aws eks list-clusters".to_string(),
            "aws iam list-users".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_provider_type() {
        let provider = AWSProvider::new();
        assert_eq!(provider.provider_type(), CloudProviderType::AWS);
    }

    #[test]
    fn test_validate_command() {
        let provider = AWSProvider::new();
        assert!(provider.validate_command("aws s3 ls").is_ok());
        assert!(provider.validate_command("gcloud compute instances list").is_err());
    }

    #[test]
    fn test_get_rag_context() {
        let provider = AWSProvider::new();
        let context = provider.get_rag_context();
        assert!(context.contains("aws"));
        assert!(context.contains("ec2"));
    }

    #[test]
    fn test_default_provider() {
        let provider = AWSProvider::default();
        assert_eq!(provider.provider_type(), CloudProviderType::AWS);
    }

    #[test]
    fn test_with_config() {
        let config = AWSConfig {
            region: Some("us-west-2".to_string()),
            profile: Some("production".to_string()),
        };
        let provider = AWSProvider::with_config(config.clone());
        assert_eq!(provider.config.region, config.region);
        assert_eq!(provider.config.profile, config.profile);
    }

    #[test]
    fn test_command_patterns() {
        let provider = AWSProvider::new();
        let patterns = provider.get_command_patterns();
        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.contains("ec2")));
        assert!(patterns.iter().any(|p| p.contains("s3")));
    }

    #[test]
    fn test_rag_context_keywords() {
        let provider = AWSProvider::new();
        let context = provider.get_rag_context();
        assert!(context.contains("Lambda"));
        assert!(context.contains("S3"));
        assert!(context.contains("EKS"));
    }
}
