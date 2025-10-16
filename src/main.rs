use anyhow::Result;
use clap::Parser;
use colored::*;
use std::sync::Arc;

// Import from our modular crates
use cuc_core::{LLMProvider, RAGEngine, VectorStore, CloudProviderType, detect_provider_from_query};
use cuc_watsonx::WatsonxClient;
use cuc_rag::{LocalVectorStore, LocalDocumentIndexer, LocalRAGEngine};
use cuc_cli::{
    CommandTranslator, CommandLearningEngine, QualityAnalyzer,
    display_banner, handle_input_with_history, print_help,
    confirm_execution, execute_command, handle_learning,
};

#[derive(Parser)]
#[command(name = "cuc")]
#[command(about = "AI-powered Cloud Universal CLI assistant", long_about = None)]
struct Cli {
    /// Direct command to execute
    #[arg(short, long)]
    command: Option<String>,
    
    /// Cloud provider (ibmcloud, aws, gcp, azure)
    #[arg(short, long)]
    provider: Option<String>,
    
    /// List supported cloud providers
    #[arg(long)]
    list_providers: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    // Handle list providers command
    if cli.list_providers {
        println!("{}", "Supported Cloud Providers:".bold());
        for provider in CloudProviderType::all() {
            println!("  {} - {}", provider.cli_command().green(), provider.display_name());
        }
        return Ok(());
    }

    // Parse cloud provider if specified
    let default_provider = if let Some(ref provider_str) = cli.provider {
        CloudProviderType::from_str(provider_str)
            .ok_or_else(|| anyhow::anyhow!("Unknown cloud provider: {}", provider_str))?
    } else {
        CloudProviderType::IBMCloud // Default to IBM Cloud for now
    };

    println!("{} Default provider: {}", "ℹ️".cyan(), default_provider);

    // Initialize components
    let mut watsonx = WatsonxClient::from_env()?;
    watsonx.connect().await?;

    // Initialize vector store and RAG
    let mut vector_store = LocalVectorStore::new();
    vector_store.connect().await?;
    let vector_store = Arc::new(vector_store);

    let document_indexer = Arc::new(LocalDocumentIndexer::new(vector_store.clone()));
    let mut rag_engine = LocalRAGEngine::new(vector_store.clone(), document_indexer.clone());

    // Initialize RAG engine
    match rag_engine.initialize().await {
        Ok(_) => println!("✅ RAG engine initialized"),
        Err(e) => println!("⚠️  RAG initialization failed: {}. Continuing without RAG.", e),
    }

    let translator = CommandTranslator::with_rag(watsonx, rag_engine);
    let mut learning_engine = CommandLearningEngine::new("command_corrections.json")?;
    let quality_analyzer = QualityAnalyzer::new();

    // Handle direct command execution
    if let Some(cmd) = cli.command {
        let result = translator.translate(&cmd).await?;
        println!("{}", result);
        return Ok(());
    }

    // Interactive mode
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

        if input_lower.starts_with("exec ") {
            let cmd = input[5..].trim();
            execute_command(cmd).await?;
            continue;
        }

        // Check for learned commands
        if let Some(learned) = learning_engine.get_learned_command(&input) {
            println!("{} Found learned command", "💡".cyan());
            println!("{} {}", "→".green(), learned.correct_command);
            
            if confirm_execution(&learned.correct_command).await? {
                execute_command(&learned.correct_command).await?;
            }
            continue;
        }

        // Detect cloud provider from query
        let detected_provider = detect_provider_from_query(&input);
        let active_provider = if let Some(detection) = detected_provider {
            println!("{} Detected provider: {} (confidence: {:.0}%)", 
                "🔍".cyan(), detection.provider, detection.confidence * 100.0);
            detection.provider
        } else {
            default_provider
        };

        // Translate natural language to command
        println!("{} Translating for {}...", "🤖".blue(), active_provider);
        
        match translator.translate(&input).await {
            Ok(command) => {
                let analysis = quality_analyzer.analyze(&command);
                
                println!("{} {}", "→".green(), command.bold());
                
                if analysis.score < 0.6 {
                    println!("{} Quality score: {:.1}%", "⚠️".yellow(), analysis.score * 100.0);
                    for issue in &analysis.issues {
                        println!("  {} {}", "•".yellow(), issue);
                    }
                }

                if confirm_execution(&command).await? {
                    let success = execute_command(&command).await?;
                    
                    if !success {
                        handle_learning(&input, &command, &mut learning_engine).await?;
                    }
                }
            }
            Err(e) => {
                println!("{} Translation failed: {}", "❌".red(), e);
            }
        }
    }

    Ok(())
}

