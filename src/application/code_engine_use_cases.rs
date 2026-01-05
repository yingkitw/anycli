//! Code Engine deployment use cases

use crate::domain::code_engine::{
    CodeEngineDeploymentConfig, CodeEngineDeploymentResult, CodeEngineDeploymentService,
};
use std::path::PathBuf;

/// Use case: Deploy application to Code Engine
pub struct DeployToCodeEngineUseCase<'a, S: CodeEngineDeploymentService> {
    deployment_service: &'a S,
}

impl<'a, S: CodeEngineDeploymentService> DeployToCodeEngineUseCase<'a, S> {
    pub fn new(deployment_service: &'a S) -> Self {
        Self { deployment_service }
    }

    /// Execute the deployment use case
    pub async fn execute(
        &self,
        config: &CodeEngineDeploymentConfig,
    ) -> Result<CodeEngineDeploymentResult, String> {
        // Step 1: Check plugin installation
        if !self.deployment_service.check_plugin_installed().await? {
            return Err("Code Engine plugin not installed. Please install it first.".to_string());
        }

        // Step 2: Ensure IBM Cloud setup
        self.deployment_service
            .ensure_setup(&config.region, &config.resource_group)
            .await?;

        // Step 3: Ensure project exists
        self.deployment_service
            .ensure_project(&config.project_name)
            .await?;

        // Step 4: Ensure secrets are set up
        if let Some(ref env_file) = config.env_file_path {
            if env_file.exists() {
                self.deployment_service
                    .ensure_secrets(&config.secret_name, env_file)
                    .await?;
            }
        }

        // Step 5: Deploy the application
        self.deployment_service.deploy(config).await
    }
}

