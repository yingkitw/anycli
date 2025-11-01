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

use core::{LLMProvider, RAGEngine, VectorStore, CloudProviderType};
use watsonx_adapter::create_watsonx_client;
use rag::{LocalVectorStore, LocalDocumentIndexer, LocalRAGEngine};
use cli::{
    CommandTranslator, CommandLearningEngine,
    display_banner, handle_input_with_history, print_help,
    confirm_execution, execute_command_with_provider, handle_learning,
};

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

    Ok(())
}
