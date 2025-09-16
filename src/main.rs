use anyhow::Result;
use clap::Parser;
use colored::*;
use std::process::Command;
use std::io::{self, Write};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};

mod watsonx;
mod translator;

use watsonx::WatsonxAI;
use translator::CommandTranslator;

/// Display startup banner
fn display_banner() {
    println!();
    println!("{}", "┌─────────────────────────────────────────────────────────────────┐".blue());
    println!("{}", "│                                                                 │".blue());
    println!("│  {}  {}                                    │", "IBM Cloud".blue().bold(), "AI CLI".green().bold());
    println!("{}", "│                                                                 │".blue());
    println!("{}", "│  🤖 AI-Powered Command Line Assistant                          │".blue());
    println!("{}", "│                                                                 │".blue());
    println!("{}", "│  Features:                                                      │".blue());
    println!("{}", "│  • 🚀 Natural language to IBM Cloud commands                   │".blue());
    println!("{}", "│  • 🔧 Intelligent error handling & suggestions                 │".blue());
    println!("{}", "│  • 📝 Interactive command editing (Esc to cancel)              │".blue());
    println!("{}", "│  • ⬆️  Command history navigation (↑/↓ arrows)                  │".blue());
    println!("{}", "│                                                                 │".blue());
    println!("│  {} {}                                        │", "v0.1.0".dimmed(), "• Powered by watsonx.ai".dimmed());
    println!("{}", "└─────────────────────────────────────────────────────────────────┘".blue());
    println!();
}

/// Handle input with command history navigation
async fn handle_input_with_history(history: &mut Vec<String>) -> Result<String> {
    enable_raw_mode()?;
    let mut input = String::new();
    let mut history_index: Option<usize> = None;
    let mut cursor_pos = 0;
    
    print!("{} ", "ibmcloud-ai>".green().bold());
    io::stdout().flush()?;
    
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Up => {
                        if !history.is_empty() {
                            let new_index = match history_index {
                                None => history.len() - 1,
                                Some(idx) => if idx > 0 { idx - 1 } else { 0 },
                            };
                            history_index = Some(new_index);
                            input = history[new_index].clone();
                            cursor_pos = input.len();
                            
                            // Clear current line and redraw
                            print!("\r{} {}{}", "ibmcloud-ai>".green().bold(), input, " ".repeat(20));
                            print!("\r{} {}", "ibmcloud-ai>".green().bold(), input);
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Down => {
                        if let Some(idx) = history_index {
                            if idx < history.len() - 1 {
                                let new_index = idx + 1;
                                history_index = Some(new_index);
                                input = history[new_index].clone();
                            } else {
                                history_index = None;
                                input.clear();
                            }
                            cursor_pos = input.len();
                            
                            // Clear current line and redraw
                            print!("\r{} {}{}", "ibmcloud-ai>".green().bold(), input, " ".repeat(20));
                            print!("\r{} {}", "ibmcloud-ai>".green().bold(), input);
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Enter => {
                        disable_raw_mode()?;
                        println!();
                        let trimmed_input = input.trim().to_string();
                        if !trimmed_input.is_empty() && (history.is_empty() || history.last() != Some(&trimmed_input)) {
                            history.push(trimmed_input.clone());
                        }
                        return Ok(trimmed_input);
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            cursor_pos = cursor_pos.saturating_sub(1);
                            history_index = None; // Reset history navigation when editing
                            print!("\r{} {}{}", "ibmcloud-ai>".green().bold(), input, " ");
                            print!("\r{} {}", "ibmcloud-ai>".green().bold(), input);
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        cursor_pos += 1;
                        history_index = None; // Reset history navigation when editing
                        print!("{}", c);
                        io::stdout().flush()?;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Handle edit mode input with Esc key detection
async fn handle_edit_input(original_command: &str) -> Result<Option<String>> {
    println!("{}", "Press Enter to execute, Esc to cancel, or type your edits:".italic().dimmed());
    print!("{} {}", "edit>".yellow().bold(), original_command);
    io::stdout().flush()?;
    
    enable_raw_mode()?;
    let mut input = String::new();
    let mut cursor_pos = original_command.len();
    
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Esc => {
                        disable_raw_mode()?;
                        println!();
                        println!("{} {}", "🚫".yellow(), "Edit cancelled".yellow());
                        return Ok(None);
                    }
                    KeyCode::Enter => {
                        disable_raw_mode()?;
                        println!();
                        if input.is_empty() {
                            return Ok(Some(original_command.to_string()));
                        } else {
                            return Ok(Some(input));
                        }
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            cursor_pos = cursor_pos.saturating_sub(1);
                            print!("\r{} {}{} ", "edit>".yellow().bold(), input, " ".repeat(original_command.len().saturating_sub(input.len())));
                            print!("\r{} {}", "edit>".yellow().bold(), input);
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        cursor_pos += 1;
                        print!("{}", c);
                        io::stdout().flush()?;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Execute a command with proper login checks and output handling
async fn execute_command(command: &str) -> Result<()> {
    // Check login status before executing IBM Cloud commands
    if command.starts_with("ibmcloud") && !command.contains("login") {
        match ensure_login().await {
            Ok(_) => {},
            Err(e) => {
                println!("{} {}: {}", "❌".red(), "Login required".red(), e);
                return Ok(());
            }
        }
    }
    
    println!("{} {}", "🚀".yellow(), "Executing command...".yellow());
    
    // For interactive commands, let the user interact directly with IBM Cloud CLI
    if command.contains("login --sso") || command.contains("login") {
        println!("{} {}", "🔐".blue(), "Running interactive IBM Cloud CLI command...".blue());
        println!("{} {}", "ℹ️".cyan(), "You will interact directly with the IBM Cloud CLI.".cyan());
        
        // Execute the command with inherited stdio for direct user interaction
        let status = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .status()?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
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
    }
    
    Ok(())
}

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
    display_banner();
    println!("{} {}", "💬".blue(), "Starting IBM Cloud AI chat mode...".blue());
    println!("{}", "Type 'exit' or 'quit' to end the session.".italic());
    println!("{}", "Type 'exec <command>' to execute a command.".italic());
    println!("{}", "Use ↑/↓ arrow keys to navigate command history.".italic());
    println!();
            
            // Initialize WatsonX
            let mut watsonx = WatsonxAI::new()?;
            watsonx.connect().await?;
            
            // Create translator
            let translator = CommandTranslator::new(watsonx);
            
            // Initialize command history
            let mut command_history: Vec<String> = Vec::new();
            
            // Chat loop
            loop {
                let input = handle_input_with_history(&mut command_history).await?;
                
                if input.is_empty() {
                    continue;
                }
                
                if input == "exit" || input == "quit" {
                    println!("{} {}", "👋".blue(), "Goodbye!".blue());
                    break;
                }
                
                if input.starts_with("exec ") {
                    let command = input.trim_start_matches("exec ").trim();
                    execute_command(command).await?;
                    continue;
                }
                
                println!("{} {}", "🤔".cyan(), "Translating your request...".cyan());
                
                // Translate query
                match translator.translate(&input).await {
                    Ok(command) => {
                        println!("{} {}", "💡".green(), "Translated command:".green());
                        println!("{}", command.bold());
                        
                        // Allow editing the command with Esc to cancel
                        match handle_edit_input(&command).await? {
                            Some(edited_command) => {
                                // User confirmed or edited the command
                                execute_command(&edited_command).await?;
                            }
                            None => {
                                // User cancelled with Esc
                                continue;
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