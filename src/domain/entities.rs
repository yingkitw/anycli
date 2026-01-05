//! Domain entities

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Command entity - represents a CLI command with its metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Command {
    /// The actual command string
    pub value: String,
    /// Cloud provider this command targets
    pub provider: CloudProvider,
    /// Quality score (0.0 to 1.0)
    pub quality_score: f32,
    /// Issues found in the command
    pub issues: Vec<String>,
}

impl Command {
    /// Create a new command
    pub fn new(value: String, provider: CloudProvider) -> Self {
        Self {
            value,
            provider,
            quality_score: 0.0,
            issues: Vec::new(),
        }
    }

    /// Check if the command is valid
    pub fn is_valid(&self) -> bool {
        self.quality_score >= 0.6 && self.issues.is_empty()
    }

    /// Update quality metrics
    pub fn update_quality(&mut self, score: f32, issues: Vec<String>) {
        self.quality_score = score;
        self.issues = issues;
    }
}

/// CommandLearning entity - represents a learned correction
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandLearning {
    /// Original natural language query
    pub query: String,
    /// Corrected command
    pub correct_command: String,
    /// Error pattern that triggered this learning
    pub error_pattern: Option<String>,
    /// Timestamp when this was learned
    pub timestamp: i64,
}

impl CommandLearning {
    /// Create a new command learning entry
    pub fn new(query: String, correct_command: String, error_pattern: Option<String>) -> Self {
        Self {
            query,
            correct_command,
            error_pattern,
            timestamp: Utc::now().timestamp(),
        }
    }
}

/// CloudProvider value object
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum CloudProvider {
    IBMCloud,
    AWS,
    GCP,
    Azure,
    VMware,
}

impl CloudProvider {
    /// Get the CLI command name for this provider
    pub fn cli_command(&self) -> &'static str {
        match self {
            CloudProvider::IBMCloud => "ibmcloud",
            CloudProvider::AWS => "aws",
            CloudProvider::GCP => "gcloud",
            CloudProvider::Azure => "az",
            CloudProvider::VMware => "govc",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            CloudProvider::IBMCloud => "IBM Cloud",
            CloudProvider::AWS => "AWS",
            CloudProvider::GCP => "Google Cloud Platform",
            CloudProvider::Azure => "Microsoft Azure",
            CloudProvider::VMware => "VMware vSphere",
        }
    }
}

impl From<crate::core::CloudProviderType> for CloudProvider {
    fn from(provider: crate::core::CloudProviderType) -> Self {
        match provider {
            crate::core::CloudProviderType::IBMCloud => CloudProvider::IBMCloud,
            crate::core::CloudProviderType::AWS => CloudProvider::AWS,
            crate::core::CloudProviderType::GCP => CloudProvider::GCP,
            crate::core::CloudProviderType::Azure => CloudProvider::Azure,
            crate::core::CloudProviderType::VMware => CloudProvider::VMware,
        }
    }
}

impl From<CloudProvider> for crate::core::CloudProviderType {
    fn from(provider: CloudProvider) -> Self {
        match provider {
            CloudProvider::IBMCloud => crate::core::CloudProviderType::IBMCloud,
            CloudProvider::AWS => crate::core::CloudProviderType::AWS,
            CloudProvider::GCP => crate::core::CloudProviderType::GCP,
            CloudProvider::Azure => crate::core::CloudProviderType::Azure,
            CloudProvider::VMware => crate::core::CloudProviderType::VMware,
        }
    }
}

