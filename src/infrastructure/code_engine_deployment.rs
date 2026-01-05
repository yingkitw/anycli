//! Infrastructure implementation of Code Engine deployment service

use crate::domain::code_engine::{
    CodeEngineDeploymentConfig, CodeEngineDeploymentResult, CodeEngineDeploymentService,
};
use std::path::PathBuf;
use std::process::Command;
use std::fs;
use tempfile::TempDir;

/// Infrastructure implementation of CodeEngineDeploymentService
pub struct CodeEngineDeploymentServiceImpl;

impl CodeEngineDeploymentServiceImpl {
    pub fn new() -> Self {
        Self
    }

    /// Retry a command with exponential backoff
    async fn retry_command<F, T>(max_attempts: u32, delay_secs: u64, f: F) -> Result<T, String>
    where
        F: Fn() -> Result<T, String>,
    {
        let mut attempt = 1;
        loop {
            match f() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempt >= max_attempts {
                        return Err(format!("Failed after {} attempts: {}", max_attempts, e));
                    }
                    eprintln!("Command failed (attempt {}/{}). Retrying in {}s...", attempt, max_attempts, delay_secs);
                    tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
                    attempt += 1;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl CodeEngineDeploymentService for CodeEngineDeploymentServiceImpl {
    async fn check_plugin_installed(&self) -> Result<bool, String> {
        let output = Command::new("ibmcloud")
            .args(&["plugin", "list"])
            .output()
            .map_err(|e| format!("Failed to check plugins: {}", e))?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        Ok(output_str.contains("code-engine"))
    }

    async fn ensure_setup(&self, region: &str, resource_group: &str) -> Result<(), String> {
        // Check if logged in
        let output = Command::new("ibmcloud")
            .args(&["target"])
            .output()
            .map_err(|e| format!("Failed to check IBM Cloud target: {}", e))?;

        if !output.status.success() {
            // Try to login
            let login_output = Command::new("ibmcloud")
                .args(&["login", "--sso"])
                .output()
                .map_err(|e| format!("Failed to login: {}", e))?;

            if !login_output.status.success() {
                return Err("Failed to login to IBM Cloud".to_string());
            }
        }

        // Target region and resource group
        Self::retry_command(3, 5, || {
            let output = Command::new("ibmcloud")
                .args(&["target", "-g", resource_group, "-r", region])
                .output()
                .map_err(|e| format!("Failed to target: {}", e))?;

            if output.status.success() {
                Ok(())
            } else {
                Err("Failed to target region and resource group".to_string())
            }
        })
        .await?;

        Ok(())
    }

    async fn ensure_project(&self, project_name: &str) -> Result<(), String> {
        // Check if plugin is installed
        if !self.check_plugin_installed().await? {
            // Install plugin
            Self::retry_command(3, 5, || {
                let output = Command::new("ibmcloud")
                    .args(&["plugin", "install", "code-engine", "-f"])
                    .output()
                    .map_err(|e| format!("Failed to install plugin: {}", e))?;

                if output.status.success() {
                    Ok(())
                } else {
                    Err("Failed to install Code Engine plugin".to_string())
                }
            })
            .await?;
        }

        // Select project
        Self::retry_command(3, 5, || {
            let output = Command::new("ibmcloud")
                .args(&["ce", "project", "select", "--name", project_name])
                .output()
                .map_err(|e| format!("Failed to select project: {}", e))?;

            if output.status.success() {
                Ok(())
            } else {
                // Project might not exist, but we'll let the deployment handle that
                Err(format!("Project '{}' not found or not accessible", project_name))
            }
        })
        .await
        .or_else(|_| {
            // If selection fails, list available projects
            let output = Command::new("ibmcloud")
                .args(&["ce", "project", "list"])
                .output()
                .map_err(|e| format!("Failed to list projects: {}", e))?;
            
            eprintln!("Available projects:");
            eprintln!("{}", String::from_utf8_lossy(&output.stdout));
            Err(format!("Please create or select project '{}' first", project_name))
        })
    }

    async fn ensure_secrets(
        &self,
        secret_name: &str,
        env_file_path: &PathBuf,
    ) -> Result<(), String> {
        if !env_file_path.exists() {
            return Ok(()); // No .env file, skip
        }

        // Try to update first, then create if it doesn't exist
        let update_output = Command::new("ibmcloud")
            .args(&[
                "ce",
                "secret",
                "update",
                "--name",
                secret_name,
                "--from-env-file",
                env_file_path.to_str().unwrap(),
            ])
            .output();

        if update_output.is_ok() && update_output.unwrap().status.success() {
            return Ok(());
        }

        // Try to create
        let create_output = Command::new("ibmcloud")
            .args(&[
                "ce",
                "secret",
                "create",
                "--name",
                secret_name,
                "--from-env-file",
                env_file_path.to_str().unwrap(),
            ])
            .output()
            .map_err(|e| format!("Failed to create secret: {}", e))?;

        if create_output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Failed to create/update secret: {}",
                String::from_utf8_lossy(&create_output.stderr)
            ))
        }
    }

    async fn deploy(&self, config: &CodeEngineDeploymentConfig) -> Result<CodeEngineDeploymentResult, String> {
        // Create temporary directory for packaging
        let temp_dir = TempDir::new()
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;
        let temp_path = temp_dir.path();

        // Copy source files
        let temp_path_buf = PathBuf::from(temp_path);
        copy_source_files(&config.source_path, &temp_path_buf)?;

        // Create Dockerfile if not provided
        if let Some(ref df_path) = config.dockerfile_path {
            if df_path.exists() {
                fs::copy(df_path, temp_path.join("Dockerfile"))
                    .map_err(|e| format!("Failed to copy Dockerfile: {}", e))?;
            } else {
                return Err("Dockerfile path does not exist".to_string());
            }
        } else {
            // Generate a basic Dockerfile
            generate_dockerfile(&temp_path_buf)?;
        }

        // Check if application exists
        let check_output = Command::new("ibmcloud")
            .args(&["ce", "application", "get", "--name", &config.app_name, "--output", "json"])
            .output()
            .map_err(|e| format!("Failed to check application: {}", e))?;

        let app_exists = check_output.status.success() 
            && String::from_utf8_lossy(&check_output.stdout).contains(&format!("\"name\":\"{}\"", config.app_name));

        // Build command arguments
        let build_timeout_str = config.build_timeout.to_string();
        let min_scale_str = config.min_scale.to_string();
        let max_scale_str = config.max_scale.to_string();
        let port_str = config.port.to_string();
        let temp_path_str = temp_path.to_str().unwrap().to_string();
        
        let mut args = vec![
            "ce",
            "application",
            if app_exists { "update" } else { "create" },
            "--name",
            &config.app_name,
            "--build-source",
            &temp_path_str,
            "--strategy",
            "dockerfile",
            "--build-size",
            &config.build_size,
            "--build-timeout",
            &build_timeout_str,
            "--env-from-secret",
            &config.secret_name,
            "--env",
            "NODE_ENV=production",
            "--cpu",
            &config.cpu,
            "--memory",
            &config.memory,
            "--min-scale",
            &min_scale_str,
            "--max-scale",
            &max_scale_str,
            "--port",
            &port_str,
            "--wait",
        ];

        // Execute deployment
        println!("🚀 Deploying to Code Engine (remote build)...");
        let output = Command::new("ibmcloud")
            .args(&args)
            .current_dir(temp_path)
            .output()
            .map_err(|e| format!("Failed to deploy: {}", e))?;

        if !output.status.success() {
            return Ok(CodeEngineDeploymentResult::failure(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // Get build run name
        let build_run_output = Command::new("ibmcloud")
            .args(&["ce", "buildrun", "list", "--output", "json"])
            .output()
            .map_err(|e| format!("Failed to get build runs: {}", e))?;

        let build_run_name = extract_build_run_name(&String::from_utf8_lossy(&build_run_output.stdout));

        // Get application URL
        let url_output = Command::new("ibmcloud")
            .args(&["ce", "application", "get", "--name", &config.app_name, "-o", "url"])
            .output()
            .map_err(|e| format!("Failed to get application URL: {}", e))?;

        let app_url = if url_output.status.success() {
            Some(String::from_utf8_lossy(&url_output.stdout).trim().to_string())
        } else {
            None
        };

        Ok(CodeEngineDeploymentResult::success(
            app_url.unwrap_or_else(|| "URL not available".to_string()),
            build_run_name.unwrap_or_else(|| "unknown".to_string()),
        ))
    }
}

fn copy_source_files(source: &PathBuf, dest: &PathBuf) -> Result<(), String> {
    // This is a simplified version - in production, you'd want more sophisticated copying
    if source.is_dir() {
        // Copy directory structure
        fs::create_dir_all(dest).map_err(|e| format!("Failed to create dest dir: {}", e))?;
        
        // For now, we'll just copy common files
        let common_files = ["package.json", "tsconfig.json", "vite.config.ts", "index.html", "bun.lockb"];
        for file in &common_files {
            let src_file = source.join(file);
            if src_file.exists() {
                fs::copy(&src_file, dest.join(file))
                    .map_err(|e| format!("Failed to copy {}: {}", file, e))?;
            }
        }

        // Copy src directory if it exists
        let src_dir = source.join("src");
        if src_dir.exists() {
            copy_dir_all(&src_dir, &dest.join("src"))
                .map_err(|e| format!("Failed to copy src directory: {}", e))?;
        }
    } else {
        return Err("Source path must be a directory".to_string());
    }
    Ok(())
}

fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }
    Ok(())
}

fn generate_dockerfile(dest: &PathBuf) -> Result<(), String> {
    let dockerfile_content = r#"FROM oven/bun:1 AS base
WORKDIR /app

# Install dependencies
COPY package.json bun.lockb* ./
RUN bun install --frozen-lockfile

# Copy source code
COPY . .

# Build the application
RUN bun run build

# Production stage
FROM oven/bun:1-slim
WORKDIR /app

# Copy built application
COPY --from=base /app/dist ./dist
COPY --from=base /app/package.json ./
COPY --from=base /app/node_modules ./node_modules

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=40s --retries=3 \
  CMD bun run -e "fetch('http://localhost:8000/health').then(r=>r.ok?process.exit(0):process.exit(1)).catch(()=>process.exit(1))"

# Start the server
CMD ["bun", "run", "dist/index.js"]
"#;

    fs::write(dest.join("Dockerfile"), dockerfile_content)
        .map_err(|e| format!("Failed to write Dockerfile: {}", e))?;
    Ok(())
}

fn extract_build_run_name(json_output: &str) -> Option<String> {
    // Simple extraction - in production, use proper JSON parsing
    if let Some(start) = json_output.find("\"name\":\"") {
        let start = start + 7;
        if let Some(end) = json_output[start..].find('"') {
            return Some(json_output[start..start + end].to_string());
        }
    }
    None
}

