//! Cloud provider abstraction for multi-cloud support

use crate::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Supported cloud providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CloudProviderType {
    /// IBM Cloud
    IBMCloud,
    /// Amazon Web Services
    AWS,
    /// Google Cloud Platform
    GCP,
    /// Microsoft Azure
    Azure,
    /// VMware vSphere/Cloud
    VMware,
}

impl CloudProviderType {
    /// Get the CLI command name for this provider
    pub fn cli_command(&self) -> &'static str {
        match self {
            CloudProviderType::IBMCloud => "ibmcloud",
            CloudProviderType::AWS => "aws",
            CloudProviderType::GCP => "gcloud",
            CloudProviderType::Azure => "az",
            CloudProviderType::VMware => "govc",
        }
    }

    /// Get the display name for this provider
    pub fn display_name(&self) -> &'static str {
        match self {
            CloudProviderType::IBMCloud => "IBM Cloud",
            CloudProviderType::AWS => "AWS",
            CloudProviderType::GCP => "Google Cloud Platform",
            CloudProviderType::Azure => "Microsoft Azure",
            CloudProviderType::VMware => "VMware vSphere",
        }
    }

    /// Get all supported providers
    pub fn all() -> Vec<CloudProviderType> {
        vec![
            CloudProviderType::IBMCloud,
            CloudProviderType::AWS,
            CloudProviderType::GCP,
            CloudProviderType::Azure,
            CloudProviderType::VMware,
        ]
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<CloudProviderType> {
        match s.to_lowercase().as_str() {
            "ibmcloud" | "ibm" => Some(CloudProviderType::IBMCloud),
            "aws" | "amazon" => Some(CloudProviderType::AWS),
            "gcp" | "gcloud" | "google" => Some(CloudProviderType::GCP),
            "azure" | "az" | "microsoft" => Some(CloudProviderType::Azure),
            "vmware" | "vsphere" | "govc" | "vmc" => Some(CloudProviderType::VMware),
            _ => None,
        }
    }
}

impl std::fmt::Display for CloudProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Cloud provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudProviderConfig {
    /// Provider type
    pub provider: CloudProviderType,
    /// Whether this provider is enabled
    pub enabled: bool,
    /// Default region (optional)
    pub default_region: Option<String>,
    /// Additional provider-specific configuration
    pub extra_config: std::collections::HashMap<String, String>,
}

impl CloudProviderConfig {
    /// Create a new cloud provider configuration
    pub fn new(provider: CloudProviderType) -> Self {
        Self {
            provider,
            enabled: true,
            default_region: None,
            extra_config: std::collections::HashMap::new(),
        }
    }

    /// Set the default region
    pub fn with_region(mut self, region: String) -> Self {
        self.default_region = Some(region);
        self
    }

    /// Add extra configuration
    pub fn with_config(mut self, key: String, value: String) -> Self {
        self.extra_config.insert(key, value);
        self
    }
}

/// Trait for cloud provider-specific operations
#[async_trait]
pub trait CloudProvider: Send + Sync {
    /// Get the provider type
    fn provider_type(&self) -> CloudProviderType;

    /// Check if the CLI is installed
    async fn is_cli_installed(&self) -> Result<bool>;

    /// Check if the user is authenticated
    async fn is_authenticated(&self) -> Result<bool>;

    /// Get provider-specific context for RAG
    fn get_rag_context(&self) -> String;

    /// Validate a command for this provider
    fn validate_command(&self, command: &str) -> Result<()>;

    /// Get common command patterns for this provider
    fn get_command_patterns(&self) -> Vec<String>;
}

/// Cloud provider detection result
#[derive(Debug, Clone)]
pub struct ProviderDetectionResult {
    /// Detected provider
    pub provider: CloudProviderType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Reason for detection
    pub reason: String,
}

/// Detect cloud provider from user query
pub fn detect_provider_from_query(query: &str) -> Option<ProviderDetectionResult> {
    let query_lower = query.to_lowercase();

    // IBM Cloud keywords
    if query_lower.contains("ibmcloud")
        || query_lower.contains("ibm cloud")
        || query_lower.contains("watson")
        || query_lower.contains("code engine")
    {
        return Some(ProviderDetectionResult {
            provider: CloudProviderType::IBMCloud,
            confidence: 0.9,
            reason: "Query contains IBM Cloud specific keywords".to_string(),
        });
    }

    // AWS keywords
    if query_lower.contains("ec2")
        || query_lower.contains("s3")
        || query_lower.contains("lambda")
        || query_lower.contains("eks")
        || query_lower.contains("aws")
    {
        return Some(ProviderDetectionResult {
            provider: CloudProviderType::AWS,
            confidence: 0.9,
            reason: "Query contains AWS specific keywords".to_string(),
        });
    }

    // GCP keywords
    if query_lower.contains("gcloud")
        || query_lower.contains("gcp")
        || query_lower.contains("compute engine")
        || query_lower.contains("gke")
        || query_lower.contains("cloud storage")
    {
        return Some(ProviderDetectionResult {
            provider: CloudProviderType::GCP,
            confidence: 0.9,
            reason: "Query contains GCP specific keywords".to_string(),
        });
    }

    // Azure keywords
    if query_lower.contains("azure")
        || query_lower.contains("az ")
        || query_lower.contains("aks")
        || query_lower.contains("virtual machine")
    {
        return Some(ProviderDetectionResult {
            provider: CloudProviderType::Azure,
            confidence: 0.9,
            reason: "Query contains Azure specific keywords".to_string(),
        });
    }

    // VMware keywords
    if query_lower.contains("vmware")
        || query_lower.contains("vsphere")
        || query_lower.contains("govc")
        || query_lower.contains("esxi")
        || query_lower.contains("vcenter")
        || query_lower.contains("vmc")
    {
        return Some(ProviderDetectionResult {
            provider: CloudProviderType::VMware,
            confidence: 0.9,
            reason: "Query contains VMware specific keywords".to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_cli_command() {
        assert_eq!(CloudProviderType::IBMCloud.cli_command(), "ibmcloud");
        assert_eq!(CloudProviderType::AWS.cli_command(), "aws");
        assert_eq!(CloudProviderType::GCP.cli_command(), "gcloud");
        assert_eq!(CloudProviderType::Azure.cli_command(), "az");
        assert_eq!(CloudProviderType::VMware.cli_command(), "govc");
    }

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(
            CloudProviderType::from_str("ibmcloud"),
            Some(CloudProviderType::IBMCloud)
        );
        assert_eq!(
            CloudProviderType::from_str("aws"),
            Some(CloudProviderType::AWS)
        );
        assert_eq!(
            CloudProviderType::from_str("gcp"),
            Some(CloudProviderType::GCP)
        );
        assert_eq!(
            CloudProviderType::from_str("azure"),
            Some(CloudProviderType::Azure)
        );
        assert_eq!(
            CloudProviderType::from_str("vmware"),
            Some(CloudProviderType::VMware)
        );
        assert_eq!(
            CloudProviderType::from_str("vsphere"),
            Some(CloudProviderType::VMware)
        );
        assert_eq!(CloudProviderType::from_str("unknown"), None);
    }

    #[test]
    fn test_detect_provider_from_query() {
        let result = detect_provider_from_query("list my ec2 instances");
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, CloudProviderType::AWS);

        let result = detect_provider_from_query("show gke clusters");
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, CloudProviderType::GCP);

        let result = detect_provider_from_query("list azure virtual machines");
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, CloudProviderType::Azure);

        let result = detect_provider_from_query("show watson services");
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, CloudProviderType::IBMCloud);

        let result = detect_provider_from_query("list vsphere vms");
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, CloudProviderType::VMware);

        let result = detect_provider_from_query("show vcenter hosts");
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, CloudProviderType::VMware);
    }

    #[test]
    fn test_cloud_provider_config() {
        let config = CloudProviderConfig::new(CloudProviderType::AWS)
            .with_region("us-east-1".to_string())
            .with_config("profile".to_string(), "default".to_string());

        assert_eq!(config.provider, CloudProviderType::AWS);
        assert_eq!(config.default_region, Some("us-east-1".to_string()));
        assert_eq!(
            config.extra_config.get("profile"),
            Some(&"default".to_string())
        );
    }

    #[test]
    fn test_provider_type_display() {
        assert_eq!(CloudProviderType::IBMCloud.to_string(), "IBM Cloud");
        assert_eq!(CloudProviderType::AWS.to_string(), "AWS");
        assert_eq!(CloudProviderType::GCP.to_string(), "Google Cloud Platform");
        assert_eq!(CloudProviderType::Azure.to_string(), "Microsoft Azure");
        assert_eq!(CloudProviderType::VMware.to_string(), "VMware vSphere");
    }

    #[test]
    fn test_provider_type_all() {
        let all = CloudProviderType::all();
        assert_eq!(all.len(), 5);
        assert!(all.contains(&CloudProviderType::IBMCloud));
        assert!(all.contains(&CloudProviderType::AWS));
        assert!(all.contains(&CloudProviderType::GCP));
        assert!(all.contains(&CloudProviderType::Azure));
        assert!(all.contains(&CloudProviderType::VMware));
    }

    #[test]
    fn test_provider_from_str_case_insensitive() {
        assert_eq!(
            CloudProviderType::from_str("IBM"),
            Some(CloudProviderType::IBMCloud)
        );
        assert_eq!(
            CloudProviderType::from_str("AMAZON"),
            Some(CloudProviderType::AWS)
        );
        assert_eq!(
            CloudProviderType::from_str("GOOGLE"),
            Some(CloudProviderType::GCP)
        );
        assert_eq!(
            CloudProviderType::from_str("MICROSOFT"),
            Some(CloudProviderType::Azure)
        );
        assert_eq!(
            CloudProviderType::from_str("VSPHERE"),
            Some(CloudProviderType::VMware)
        );
    }

    #[test]
    fn test_detect_provider_no_match() {
        let result = detect_provider_from_query("some random text");
        assert!(result.is_none());
    }

    #[test]
    fn test_detection_result_confidence() {
        let result = detect_provider_from_query("list ec2 instances").unwrap();
        assert_eq!(result.confidence, 0.9);
        assert!(!result.reason.is_empty());
    }

    #[test]
    fn test_cloud_provider_config_default() {
        let config = CloudProviderConfig::new(CloudProviderType::GCP);
        assert_eq!(config.provider, CloudProviderType::GCP);
        assert_eq!(config.default_region, None);
        assert!(config.extra_config.is_empty());
        assert!(config.enabled);
    }

    #[test]
    fn test_cloud_provider_config_chaining() {
        let config = CloudProviderConfig::new(CloudProviderType::Azure)
            .with_region("eastus".to_string())
            .with_config("subscription".to_string(), "sub-123".to_string())
            .with_config("resource_group".to_string(), "rg-prod".to_string());

        assert_eq!(config.default_region, Some("eastus".to_string()));
        assert_eq!(config.extra_config.len(), 2);
    }
}
