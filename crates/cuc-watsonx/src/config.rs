//! WatsonX configuration

use serde::{Deserialize, Serialize};
use std::env;
use cuc_core::{Error, Result};

/// Configuration for WatsonX AI client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatsonxConfig {
    pub api_key: String,
    pub project_id: String,
    pub iam_url: String,
    pub api_url: String,
}

impl WatsonxConfig {
    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();

        let api_key = env::var("WATSONX_API_KEY")
            .or_else(|_| env::var("API_KEY"))
            .map_err(|_| Error::Configuration(
                "WATSONX_API_KEY or API_KEY environment variable not found".to_string()
            ))?;

        let project_id = env::var("WATSONX_PROJECT_ID")
            .or_else(|_| env::var("PROJECT_ID"))
            .map_err(|_| Error::Configuration(
                "WATSONX_PROJECT_ID or PROJECT_ID environment variable not found".to_string()
            ))?;

        let iam_url = env::var("IAM_IBM_CLOUD_URL")
            .unwrap_or_else(|_| "iam.cloud.ibm.com".to_string());

        let api_url = env::var("WATSONX_API_URL")
            .unwrap_or_else(|_| "https://us-south.ml.cloud.ibm.com".to_string());

        Ok(Self {
            api_key,
            project_id,
            iam_url,
            api_url,
        })
    }

    /// Create configuration with explicit values
    pub fn new(api_key: String, project_id: String) -> Self {
        Self {
            api_key,
            project_id,
            iam_url: "iam.cloud.ibm.com".to_string(),
            api_url: "https://us-south.ml.cloud.ibm.com".to_string(),
        }
    }
}
