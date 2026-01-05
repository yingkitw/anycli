//! Code Engine deployment domain entities and value objects

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Code Engine deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeEngineDeploymentConfig {
    /// Project name
    pub project_name: String,
    /// Application name
    pub app_name: String,
    /// Region (e.g., "us-south")
    pub region: String,
    /// Resource group
    pub resource_group: String,
    /// Source directory path
    pub source_path: PathBuf,
    /// Dockerfile path (optional, will be generated if not provided)
    pub dockerfile_path: Option<PathBuf>,
    /// Environment variables from .env file
    pub env_file_path: Option<PathBuf>,
    /// Secret name for credentials
    pub secret_name: String,
    /// CPU allocation
    pub cpu: String,
    /// Memory allocation
    pub memory: String,
    /// Minimum scale
    pub min_scale: u32,
    /// Maximum scale
    pub max_scale: u32,
    /// Port number
    pub port: u16,
    /// Build size (small, medium, large, xlarge, xxlarge)
    pub build_size: String,
    /// Build timeout in seconds
    pub build_timeout: u32,
}

impl Default for CodeEngineDeploymentConfig {
    fn default() -> Self {
        Self {
            project_name: "watsonx-sdlc-project".to_string(),
            app_name: "watsonx-sdlc-bun".to_string(),
            region: "us-south".to_string(),
            resource_group: "Default".to_string(),
            source_path: PathBuf::from("."),
            dockerfile_path: None,
            env_file_path: Some(PathBuf::from(".env")),
            secret_name: "watsonx-credentials".to_string(),
            cpu: "1".to_string(),
            memory: "4G".to_string(),
            min_scale: 1,
            max_scale: 3,
            port: 8000,
            build_size: "large".to_string(),
            build_timeout: 900,
        }
    }
}

/// Code Engine deployment result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CodeEngineDeploymentResult {
    /// Whether deployment was successful
    pub success: bool,
    /// Application URL if deployment succeeded
    pub app_url: Option<String>,
    /// Build run name
    pub build_run_name: Option<String>,
    /// Error message if deployment failed
    pub error: Option<String>,
    /// Deployment logs
    pub logs: Vec<String>,
}

impl CodeEngineDeploymentResult {
    pub fn success(url: String, build_run: String) -> Self {
        Self {
            success: true,
            app_url: Some(url),
            build_run_name: Some(build_run),
            error: None,
            logs: Vec::new(),
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            app_url: None,
            build_run_name: None,
            error: Some(error),
            logs: Vec::new(),
        }
    }
}

/// Domain service for Code Engine deployment
#[async_trait::async_trait]
pub trait CodeEngineDeploymentService {
    /// Deploy an application to Code Engine
    async fn deploy(
        &self,
        config: &CodeEngineDeploymentConfig,
    ) -> Result<CodeEngineDeploymentResult, String>;

    /// Check if Code Engine plugin is installed
    async fn check_plugin_installed(&self) -> Result<bool, String>;

    /// Ensure IBM Cloud is logged in and targeted
    async fn ensure_setup(
        &self,
        region: &str,
        resource_group: &str,
    ) -> Result<(), String>;

    /// Select or create a Code Engine project
    async fn ensure_project(&self, project_name: &str) -> Result<(), String>;

    /// Create or update secrets from .env file
    async fn ensure_secrets(
        &self,
        secret_name: &str,
        env_file_path: &PathBuf,
    ) -> Result<(), String>;
}

