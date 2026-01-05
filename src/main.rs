use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::sync::Arc;

// Core modules
mod core;
mod cli;
mod rag;
mod providers;
mod watsonx_adapter;

// DDD layers
mod domain;
mod application;
mod infrastructure;

use core::{LLMProvider, RAGEngine, VectorStore, CloudProviderType};
use watsonx_adapter::create_watsonx_client;
use rag::{LocalVectorStore, LocalDocumentIndexer, LocalRAGEngine};
use cli::{
    CommandTranslator, CommandLearningEngine, IntentDetector, QueryIntent,
    display_banner, handle_input_with_history, print_help,
    confirm_execution, execute_command_with_provider, handle_learning,
};
use domain::code_engine::{CodeEngineDeploymentConfig, CodeEngineDeploymentService};
use application::DeployToCodeEngineUseCase;
use infrastructure::CodeEngineDeploymentServiceImpl;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "anycli")]
#[command(about = "AI-powered Cloud Universal CLI assistant")]
#[command(version = "0.1.0")]
#[command(author = "AnyCLI Team")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Cloud provider (ibmcloud, aws, gcp, azure, vmware)
    #[arg(short, long, global = true)]
    provider: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Translate natural language to cloud command
    #[command(about = "Translate a natural language query to a cloud command")]
    Translate {
        /// The natural language query
        query: String,
    },

    /// List supported cloud providers
    #[command(about = "Show all supported cloud providers")]
    Providers,

    /// Interactive mode (default)
    #[command(about = "Start interactive mode")]
    Interactive,

    /// Deploy application to IBM Code Engine
    #[command(about = "Deploy an application to IBM Code Engine")]
    Deploy {
        /// Application name
        #[arg(short, long, default_value = "watsonx-sdlc-bun")]
        app_name: String,

        /// Project name
        #[arg(short, long, default_value = "watsonx-sdlc-project")]
        project_name: String,

        /// Source directory path
        #[arg(short, long, default_value = ".")]
        source: String,

        /// Region
        #[arg(short, long, default_value = "us-south")]
        region: String,

        /// Resource group
        #[arg(short = 'g', long, default_value = "Default")]
        resource_group: String,

        /// Port number
        #[arg(short, long, default_value = "8000")]
        port: u16,

        /// Memory allocation (e.g., "4G")
        #[arg(short, long, default_value = "4G")]
        memory: String,

        /// CPU allocation
        #[arg(short, long, default_value = "1")]
        cpu: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // Parse cloud provider if specified
    let default_provider = if let Some(ref provider_str) = cli.provider {
        CloudProviderType::from_str(provider_str)
            .ok_or_else(|| anyhow::anyhow!("Unknown cloud provider: {}", provider_str))?
    } else {
        CloudProviderType::IBMCloud
    };

    // Initialize components
    let mut watsonx = create_watsonx_client()?;
    watsonx.connect().await?;

    let mut vector_store = LocalVectorStore::new();
    vector_store.connect().await?;
    let vector_store = Arc::new(vector_store);

    let document_indexer = Arc::new(LocalDocumentIndexer::new(vector_store.clone()));
    let mut rag_engine = LocalRAGEngine::new(vector_store.clone(), document_indexer.clone());
    
    match rag_engine.initialize().await {
        Ok(_) => {},
        Err(e) => eprintln!("⚠️  RAG initialization failed: {}", e),
    }

    let translator = CommandTranslator::with_rag(watsonx, rag_engine);
    let mut learning_engine = CommandLearningEngine::new("command_corrections.json")?;

    // Handle commands
    match cli.command {
        Some(Commands::Providers) => {
            println!("{}", "Supported Cloud Providers:".bold());
            for provider in CloudProviderType::all() {
                println!("  {} - {}", provider.cli_command().green(), provider.display_name());
            }
        }
        Some(Commands::Translate { query }) => {
            match translator.translate(&query).await {
                Ok(command) => println!("{}", command),
                Err(e) => eprintln!("{} {}", "❌".red(), e),
            }
        }
        Some(Commands::Interactive) | None => {
            run_interactive(&translator, &mut learning_engine, default_provider).await?;
        }
        Some(Commands::Deploy {
            app_name,
            project_name,
            source,
            region,
            resource_group,
            port,
            memory,
            cpu,
        }) => {
            let deployment_service = CodeEngineDeploymentServiceImpl::new();
            let deploy_use_case = DeployToCodeEngineUseCase::new(&deployment_service);

            let config = CodeEngineDeploymentConfig {
                app_name,
                project_name,
                region,
                resource_group,
                source_path: PathBuf::from(source),
                dockerfile_path: None,
                env_file_path: Some(PathBuf::from(".env")),
                secret_name: "watsonx-credentials".to_string(),
                cpu,
                memory,
                min_scale: 1,
                max_scale: 3,
                port,
                build_size: "large".to_string(),
                build_timeout: 900,
            };

            println!("{} Deploying to Code Engine...", "🚀".yellow());
            match deploy_use_case.execute(&config).await {
                Ok(result) => {
                    if result.success {
                        println!("{} Deployment successful!", "✅".green());
                        if let Some(url) = result.app_url {
                            println!("{} Application URL: {}", "🌐".blue(), url.bold());
                            println!();
                            println!("{} API Endpoints:", "📡".cyan());
                            println!("  {}/health - Health check", url);
                            println!("  {}/api/generate - AI generation", url);
                            println!("  {}/api/generate-tests - Test generation", url);
                        }
                        if let Some(build_run) = result.build_run_name {
                            println!("{} Build run: {}", "🔨".yellow(), build_run);
                        }
                    } else {
                        println!("{} Deployment failed", "❌".red());
                        if let Some(error) = result.error {
                            eprintln!("{}", error);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{} Deployment error: {}", "❌".red(), e);
                }
            }
        }
    }

    Ok(())
}

async fn run_interactive(
    translator: &CommandTranslator<impl LLMProvider, impl RAGEngine>,
    learning_engine: &mut CommandLearningEngine,
    default_provider: CloudProviderType,
) -> Result<()> {
    display_banner();
    let mut history = Vec::new();
    let intent_detector = IntentDetector::new();
    let deployment_service = CodeEngineDeploymentServiceImpl::new();

    loop {
        let input = handle_input_with_history(&mut history).await?;

        if input.is_empty() {
            continue;
        }

        let input_lower = input.to_lowercase();

        // Handle special commands
        if input_lower == "exit" || input_lower == "quit" {
            println!("{}", "👋 Goodbye!".green());
            break;
        }

        if input_lower == "help" {
            print_help();
            continue;
        }

        // Detect intent
        let intent = intent_detector.detect(&input);

        match intent {
            QueryIntent::DeployToCodeEngine { app_name, project_name } => {
                println!("{} Detected deployment request", "🚀".yellow());
                
                // Use defaults or extracted values
                let config = CodeEngineDeploymentConfig {
                    app_name: app_name.unwrap_or_else(|| "watsonx-sdlc-bun".to_string()),
                    project_name: project_name.unwrap_or_else(|| "watsonx-sdlc-project".to_string()),
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
                };

                println!("{} App: {}, Project: {}", 
                    "📦".cyan(), 
                    config.app_name.bold(), 
                    config.project_name.bold()
                );

                if confirm_execution("Deploy to Code Engine").await? {
                    let deploy_use_case = DeployToCodeEngineUseCase::new(&deployment_service);
                    match deploy_use_case.execute(&config).await {
                        Ok(result) => {
                            if result.success {
                                println!("{} Deployment successful!", "✅".green());
                                if let Some(url) = result.app_url {
                                    println!("{} Application URL: {}", "🌐".blue(), url.bold());
                                }
                            } else {
                                println!("{} Deployment failed", "❌".red());
                                if let Some(error) = result.error {
                                    eprintln!("{}", error);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("{} Deployment error: {}", "❌".red(), e);
                        }
                    }
                }
            }
            QueryIntent::CommandTranslation => {
                // Translate natural language to command
                match translator.translate(&input).await {
                    Ok(command) => {
                        println!("{} {}", "→".green(), command.bold());
                        
                        if confirm_execution(&command).await? {
                            let result = execute_command_with_provider(&command, Some(default_provider)).await?;
                            
                            if !result.success {
                                println!("{} Command failed", "❌".red());
                                handle_learning(&input, &command, learning_engine).await?;
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} {}", "❌".red(), e);
                    }
                }
            }
            QueryIntent::Unknown => {
                // Fall back to command translation
                match translator.translate(&input).await {
                    Ok(command) => {
                        println!("{} {}", "→".green(), command.bold());
                        
                        if confirm_execution(&command).await? {
                            let result = execute_command_with_provider(&command, Some(default_provider)).await?;
                            
                            if !result.success {
                                println!("{} Command failed", "❌".red());
                                handle_learning(&input, &command, learning_engine).await?;
                            }
                        }
                    }
                    Err(e) => {
                        println!("{} {}", "❌".red(), e);
                    }
                }
            }
        }
    }

    Ok(())
}
