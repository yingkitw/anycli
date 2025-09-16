use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::process::Command;
use std::io::{self, Write};

mod watsonx;
mod translator;

use watsonx::WatsonxAI;
use translator::CommandTranslator;

/// Check if user is logged into IBM Cloud
async fn check_ibmcloud_login() -> Result<bool> {
    let output = Command::new("ibmcloud")
        .args(["account", "show"])
        .output()?;
    
    // If the command succeeds and doesn't contain error messages, user is logged in
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Check for common "not logged in" indicators
        let not_logged_in_indicators = [
            "not logged in",
            "Please login",
            "authentication failed",
            "FAILED",
            "Error"
        ];
        
        let output_text = format!("{} {}", stdout, stderr).to_lowercase();
        let is_logged_in = !not_logged_in_indicators.iter()
            .any(|indicator| output_text.contains(&indicator.to_lowercase()));
            
        Ok(is_logged_in)
    } else {
        Ok(false)
    }
}

/// Prompt user to login if not authenticated
async fn ensure_login() -> Result<()> {
    if !check_ibmcloud_login().await? {
        println!("{} {}", "⚠️".yellow(), "You are not logged into IBM Cloud.".yellow());
        println!("{} {}", "💡".cyan(), "Please login first using:".cyan());
        println!("  {} {}", "•".blue(), "ibmcloud login".bold());
        println!("  {} {}", "•".blue(), "ibmcloud login --sso".bold());
        println!();
        
        print!("{} ", "Would you like to login now? [Y/n]:".green().bold());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        
        if input.is_empty() || input == "y" || input == "yes" {
            println!("{} {}", "🔐".blue(), "Starting IBM Cloud login...".blue());
            
            // Ask for login method
            print!("{} ", "Use SSO login? [Y/n]:".cyan().bold());
            io::stdout().flush()?;
            
            let mut sso_input = String::new();
            io::stdin().read_line(&mut sso_input)?;
            let sso_input = sso_input.trim().to_lowercase();
            
            let login_command = if sso_input.is_empty() || sso_input == "y" || sso_input == "yes" {
                "ibmcloud login --sso"
            } else {
                "ibmcloud login"
            };
            
            println!("{} {}", "ℹ️".cyan(), "You will interact directly with the IBM Cloud CLI.".cyan());
            
            let status = Command::new("sh")
                .arg("-c")
                .arg(login_command)
                .status()?;
                
            if !status.success() {
                return Err(anyhow::anyhow!("Login failed. Please try again manually."));
            }
            
            println!("{} {}", "✅".green(), "Login completed successfully!".green());
        } else {
            return Err(anyhow::anyhow!("IBM Cloud login is required to execute commands."));
        }
    }
    
    Ok(())
}

#[derive(Parser)]
#[command(
    name = "ibmcloud-ai",
    author,
    version,
    about = "AI-powered IBM Cloud CLI that translates natural language to IBM Cloud commands",
    long_about = None
)]
struct Cli {
    // No subcommands - just start chat mode by default
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    let _cli = Cli::parse();
    
    // Always start chat mode
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
                        println!("{}", "Press Enter to execute, or type your edits:".italic().dimmed());
                        print!("{} {}", "edit>".yellow().bold(), edited_command);
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
                        // Check login status before executing IBM Cloud commands
                        if edited_command.starts_with("ibmcloud") && !edited_command.contains("login") {
                            match ensure_login().await {
                                Ok(_) => {},
                                Err(e) => {
                                    println!("{} {}: {}", "❌".red(), "Login required".red(), e);
                                    continue;
                                }
                            }
                        }
                        
                        println!("{} {}", "🚀".yellow(), "Executing command...".yellow());
                        
                        // For interactive commands, let the user interact directly with IBM Cloud CLI
                        if edited_command.contains("login --sso") || edited_command.contains("login") {
                            println!("{} {}", "🔐".blue(), "Running interactive IBM Cloud CLI command...".blue());
                            println!("{} {}", "ℹ️".cyan(), "You will interact directly with the IBM Cloud CLI.".cyan());
                            
                            // Execute the command with inherited stdio for direct user interaction
                            let status = if cfg!(target_os = "windows") {
                                Command::new("cmd")
                                    .args(["/C", &edited_command])
                                    .status()?
                            } else {
                                Command::new("sh")
                                    .arg("-c")
                                    .arg(&edited_command)
                                    .status()?
                            };
                            
                            if status.success() {
                                println!("{} {}", "✅".green(), "Command completed successfully".green());
                            } else {
                                println!("{} {}", "❌".red(), "Command failed".red());
                            }
                        } else {
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
                        }
                    },
                    Err(e) => {
                        println!("{} {}: {}", "❌".red(), "Translation failed".red(), e);
                    }
                }
            }
    
    Ok(())
}