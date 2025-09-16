use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::process::Command;
use std::io::{self, Write};

mod watsonx;
mod translator;

use watsonx::WatsonxAI;
use translator::CommandTranslator;

#[derive(Parser)]
#[command(
    name = "ibmcloud-ai",
    author,
    version,
    about = "AI-powered IBM Cloud CLI that translates natural language to IBM Cloud commands",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute an IBM Cloud command from natural language
    #[command(alias = "a")]
    Ask {
        /// Natural language query to translate to IBM Cloud command
        #[arg(required = true, num_args = 1.., last = true)]
        query: Vec<String>,
        
        /// Execute the command after translation
        #[arg(short, long)]
        execute: bool,
    },
    
    /// Start an interactive chat session with the IBM Cloud AI assistant
    #[command(alias = "c")]
    Chat,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    let cli = Cli::parse();
    
    match &cli.command {
        Commands::Ask { query, execute } => {
            let query = query.join(" ");
            
            println!("{} {}", "🤔".cyan(), "Translating your request...".cyan());
            
            // Initialize WatsonX
            let mut watsonx = WatsonxAI::new()?;
            watsonx.connect().await?;
            
            // Create translator and translate query
            let translator = CommandTranslator::new(watsonx);
            let command = translator.translate(&query).await?;
            
            println!("{} {}", "💡".green(), "Translated command:".green());
            println!("{}", command.bold());
            
            if *execute {
                println!("\n{} {}", "🚀".yellow(), "Executing command...".yellow());
                
                // Execute the command
                let output = if cfg!(target_os = "windows") {
                    Command::new("cmd")
                        .args(["/C", &command])
                        .output()?
                } else {
                    Command::new("sh")
                        .arg("-c")
                        .arg(&command)
                        .output()?
                };
                
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                
                if !stdout.is_empty() {
                    println!("{}", stdout);
                }
                
                if !stderr.is_empty() {
                    eprintln!("{}", stderr.red());
                }
                
                if !output.status.success() {
                    println!("{} {}", "❌".red(), "Command failed".red());
                } else {
                    println!("{} {}", "✅".green(), "Command executed successfully".green());
                }
            } else {
                println!("\n{} {}", "ℹ️", "To execute this command, run with --execute flag");
            }
        },
        Commands::Chat => {
            println!("{} {}", "💬".blue(), "Starting IBM Cloud AI chat mode...".blue());
            println!("{}", "Type 'exit' or 'quit' to end the session.".italic());
            println!("{}", "Type 'exec <command>' to execute a command.".italic());
            println!();
            
            // Initialize WatsonX
            let mut watsonx = WatsonxAI::new()?;
            watsonx.connect().await?;
            
            // Create translator
            let translator = CommandTranslator::new(watsonx);
            
            // Chat loop
            loop {
                print!("{} ", "ibmcloud-ai>".green().bold());
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let input = input.trim();
                
                if input.is_empty() {
                    continue;
                }
                
                if input == "exit" || input == "quit" {
                    println!("{} {}", "👋".blue(), "Goodbye!".blue());
                    break;
                }
                
                if input.starts_with("exec ") {
                    let command = input.trim_start_matches("exec ").trim();
                    println!("{} {}", "🚀".yellow(), "Executing command...".yellow());
                    
                    // Execute the command
                    let output = if cfg!(target_os = "windows") {
                        Command::new("cmd")
                            .args(["/C", command])
                            .output()?
                    } else {
                        Command::new("sh")
                            .arg("-c")
                            .arg(command)
                            .output()?
                    };
                    
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    
                    if !stdout.is_empty() {
                        println!("{}", stdout);
                    }
                    
                    if !stderr.is_empty() {
                        eprintln!("{}", stderr.red());
                    }
                    
                    if !output.status.success() {
                        println!("{} {}", "❌".red(), "Command failed".red());
                    } else {
                        println!("{} {}", "✅".green(), "Command executed successfully".green());
                    }
                    
                    continue;
                }
                
                println!("{} {}", "🤔".cyan(), "Translating your request...".cyan());
                
                // Translate query
                match translator.translate(input).await {
                    Ok(command) => {
                        println!("{} {}", "💡".green(), "Translated command:".green());
                        println!("{}", command.bold());
                        
                        // Allow editing the command
                        let mut edited_command = command.clone();
                        print!("{} ", "edit>".yellow().bold());
                        print!("{}", edited_command);
                        io::stdout().flush()?;
                        
                        // Read user input while showing the current command
                        let mut edit_input = String::new();
                        io::stdin().read_line(&mut edit_input)?;
                        
                        // If user just pressed enter, use the original command
                        // Otherwise, use their edited version
                        if edit_input.trim().is_empty() {
                            // Keep the original command
                        } else if edit_input.trim() != "end" {
                            edited_command = edit_input.trim().to_string();
                        }
                        
                        // Execute the edited command
                        println!("{} {}", "🚀".yellow(), "Executing command...".yellow());
                        
                        let output = if cfg!(target_os = "windows") {
                            Command::new("cmd")
                                .args(["/C", &edited_command])
                                .output()?
                        } else {
                            Command::new("sh")
                                .arg("-c")
                                .arg(&edited_command)
                                .output()?
                        };
                        
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        
                        if !stdout.is_empty() {
                            println!("{}", stdout);
                        }
                        
                        if !stderr.is_empty() {
                            eprintln!("{}", stderr.red());
                        }
                        
                        if !output.status.success() {
                            println!("{} {}", "❌".red(), "Command failed".red());
                        } else {
                            println!("{} {}", "✅".green(), "Command executed successfully".green());
                        }
                    },
                    Err(e) => {
                        println!("{} {}: {}", "❌".red(), "Translation failed".red(), e);
                    }
                }
            }
        }
    }
    
    Ok(())
}