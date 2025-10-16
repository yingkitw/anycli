use anyhow::Result;
use clap::Parser;
use colored::*;
use std::process::Command;
use std::io::{self, Write};
use std::sync::Arc;

// Import from our modular crates
use cuc_core::{LLMProvider, RAGEngine, RAGQuery};
use cuc_watsonx::WatsonxClient;
use cuc_rag::{LocalVectorStore, LocalDocumentIndexer, LocalRAGEngine};
use cuc_cli::{
    CommandTranslator, CommandLearningEngine, QualityAnalyzer,
    display_banner, handle_input_with_history,
};

#[derive(Parser)]
#[command(name = "icx")]
#[command(about = "AI-powered IBM Cloud CLI assistant", long_about = None)]
struct Cli {
    /// Direct command to execute
    #[arg(short, long)]
    command: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

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
    let learning_engine = CommandLearningEngine::new("command_corrections.json")?;
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

        // Translate natural language to command
        println!("{} Translating...", "🤖".blue());
        
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
                        offer_correction(&input, &command, &learning_engine).await?;
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

fn print_help() {
    println!("{}", "Available commands:".bold());
    println!("  {} - Type natural language queries to translate to IBM Cloud commands", "query".green());
    println!("  {} - Execute a command directly", "exec <command>".green());
    println!("  {} - Show this help message", "help".green());
    println!("  {} - Exit the application", "exit/quit".green());
    println!();
    println!("{}", "Examples:".bold());
    println!("  list my resource groups");
    println!("  show all kubernetes clusters");
    println!("  exec ibmcloud target --cf");
}

async fn confirm_execution(command: &str) -> Result<bool> {
    print!("{} Execute this command? [Y/n]: ", "❓".cyan());
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    let response = response.trim().to_lowercase();

    Ok(response.is_empty() || response == "y" || response == "yes")
}

async fn execute_command(command: &str) -> Result<bool> {
    println!("{} Executing...", "🚀".yellow());

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", command]).output()?
    } else {
        Command::new("sh").arg("-c").arg(command).output()?
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        println!("{}", stdout);
    }

    if !stderr.is_empty() {
        eprintln!("{}", stderr.red());
    }

    if output.status.success() {
        println!("{} Command executed successfully", "✅".green());
        Ok(true)
    } else {
        println!("{} Command failed", "❌".red());
        Ok(false)
    }
}

async fn offer_correction(
    query: &str,
    failed_command: &str,
    learning_engine: &CommandLearningEngine,
) -> Result<()> {
    println!("{} Would you like to provide the correct command?", "📝".cyan());
    print!("Correct command (or press Enter to skip): ");
    io::stdout().flush()?;

    let mut correction = String::new();
    io::stdin().read_line(&mut correction)?;
    let correction = correction.trim();

    if !correction.is_empty() {
        let mut engine = learning_engine.clone();
        engine.add_correction(
            query.to_string(),
            correction.to_string(),
            Some(format!("Failed command: {}", failed_command)),
        ).await?;
        println!("{} Thanks! I'll remember this.", "✅".green());
    }

    Ok(())
}
