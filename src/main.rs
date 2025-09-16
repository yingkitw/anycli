use anyhow::Result;
use clap::Parser;
use colored::*;
use std::process::Command;
use std::io::{self, Write, IsTerminal};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode, size},
};

mod watsonx;
mod translator;
mod vector_store;
mod document_indexer;
mod rag;
mod local_vector_store;
mod local_document_indexer;
mod local_rag;
mod command_learning;
mod quality_analyzer;

use watsonx::{WatsonxAI, RetryConfig};
use local_rag::LocalRAGEngine;
use translator::CommandTranslator;

/// Display startup banner with Carbon Design System inspired styling
fn display_banner() {
    let terminal_width = size().map(|(w, _)| w as usize).unwrap_or(80);
    let banner_width = std::cmp::min(67, terminal_width.saturating_sub(4));
    
    let top_border = format!("┌{}┐", "─".repeat(banner_width - 2));
    let bottom_border = format!("└{}┘", "─".repeat(banner_width - 2));
    let empty_line = format!("│{}│", " ".repeat(banner_width - 2));
    
    println!();
    println!("{}", top_border.blue());
    println!("{}", empty_line.blue());
    
    let title_line = format!("│  {}  {}{}│", 
        "IBM Cloud".blue().bold(), 
        "AI CLI".green().bold(),
        " ".repeat(banner_width - 20));
    println!("{}", title_line);
    
    println!("{}", empty_line.blue());
    
    let feature_lines = vec![
        "🤖 AI-Powered Command Line Assistant",
        "",
        "Features:",
        "• 🚀 Natural language to IBM Cloud commands",
        "• 🔧 Intelligent error handling & suggestions", 
        "• 📝 Interactive command editing (Esc to cancel)",
        "• ⬆️  Command history navigation (↑/↓ arrows)",
        "• 🔐 Automatic login status verification",
        "",
        "v0.1.0 • Powered by watsonx.ai"
    ];
    
    for line in feature_lines {
        if line.is_empty() {
            println!("{}", empty_line.blue());
        } else {
            let content = if line.starts_with("v0.1.0") {
                format!("│  {}{}│", line.dimmed(), " ".repeat(banner_width - line.len() - 4))
            } else {
                format!("│  {}{}│", line, " ".repeat(banner_width - line.len() - 4))
            };
            println!("{}", content.blue());
        }
    }
    
    println!("{}", empty_line.blue());
    println!("{}", bottom_border.blue());
    println!();
    println!("{}", "💡 Tip: Type your request in natural language, or 'help' for commands".dimmed());
    println!();
}

/// Handle input with command history navigation
async fn handle_input_with_history(history: &mut Vec<String>) -> Result<String> {
    // Check if stdin is a terminal (interactive) or piped
    if !io::stdin().is_terminal() {
        // Handle piped input - read from stdin directly
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_string();
        if !input.is_empty() {
            history.push(input.clone());
        }
        return Ok(input);
    }
    
    enable_raw_mode()?;
    let mut input = String::new();
    let mut history_index: Option<usize> = None;
    let mut cursor_pos = 0;
    
    // Carbon Design System: Consistent, accessible prompt with clear visual hierarchy
    print!("{} ", "ibmcloud-ai>".green().bold());
    io::stdout().flush()?;
    
    loop {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Up => {
                        // Enhanced history navigation with Carbon UX principles
                        if !history.is_empty() {
                            let new_index = match history_index {
                                None => history.len() - 1,
                                Some(idx) => if idx > 0 { idx - 1 } else { 0 },
                            };
                            history_index = Some(new_index);
                            input = history[new_index].clone();
                            cursor_pos = input.len();
                            
                            // Clear current line and redraw with improved visual feedback
                            print!("\r{} {}{}", "ibmcloud-ai>".green().bold(), input, " ".repeat(20));
                            print!("\r{} {}", "ibmcloud-ai>".green().bold(), input);
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Down => {
                        // Enhanced forward navigation in command history
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
                        
                        // Enhanced history management following Carbon principles
                        if !trimmed_input.is_empty() && (history.is_empty() || history.last() != Some(&trimmed_input)) {
                            history.push(trimmed_input.clone());
                            // Keep history manageable (Carbon principle: performance optimization)
                            if history.len() > 50 {
                                history.remove(0);
                            }
                        }
                        return Ok(trimmed_input);
                    }
                    KeyCode::Backspace => {
                        // Enhanced backspace handling with visual feedback
                        if !input.is_empty() {
                            input.pop();
                            cursor_pos = cursor_pos.saturating_sub(1);
                            history_index = None; // Reset history navigation when editing
                            print!("\r{} {}{}", "ibmcloud-ai>".green().bold(), input, " ");
                            print!("\r{} {}", "ibmcloud-ai>".green().bold(), input);
                            io::stdout().flush()?;
                        }
                    }
                    KeyCode::Esc => {
                        // Carbon UX: Clear escape behavior for better user experience
                        disable_raw_mode()?;
                        println!();
                        println!("{} {}", "❌".yellow(), "Input cancelled. Type 'exit' to quit.".yellow());
                        return Ok(String::new());
                    }
                    KeyCode::Char(c) => {
                        // Enhanced character input with immediate visual feedback
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
async fn execute_command_with_feedback(
    command: &str, 
    rag_engine: &LocalRAGEngine, 
    user_input: &str,
    translator: &mut CommandTranslator
) -> Result<bool> {
    let success = execute_command(command, rag_engine, user_input).await?;
    
    // If command failed, add feedback for future improvements
    if !success {
        let error_msg = format!("Command '{}' failed execution", command);
        translator.add_execution_feedback(command, &error_msg, user_input);
    }
    
    Ok(success)
}

async fn execute_command(command: &str, rag_engine: &LocalRAGEngine, user_input: &str) -> Result<bool> {
    // Check login status before executing IBM Cloud commands
    if command.starts_with("ibmcloud") && !command.contains("login") {
        match ensure_login().await {
            Ok(_) => {},
            Err(e) => {
                println!("{} {}: {}", "❌".red(), "Login required".red(), e);
                return Ok(false);
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
            return Ok(true);
        } else {
            println!("{} {}", "❌".red(), "Command failed".red());
            return Ok(false);
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
            
            // Check for known error patterns and suggest corrections
             if stderr.contains("not registered") || stderr.contains("Unknown command") || stderr.contains("not found") || stderr.contains("not a registered command") {
                 println!("{} {}", "💡".yellow(), "This looks like a command that might need correction.".yellow());
                 
                 // Special handling for plugin-related errors
                 if stderr.contains("not a registered command") && stderr.contains("plug-ins") {
                     println!("{} {}", "🔌".cyan(), "This appears to be a missing plugin. You may need to:".cyan());
                     println!("{} {}", "  •".cyan(), "Install the required plugin with 'ibmcloud plugin install <plugin-name>'".cyan());
                     println!("{} {}", "  •".cyan(), "Check available plugins with 'ibmcloud plugin repo-plugins'".cyan());
                     println!("{} {}", "  •".cyan(), "Or use an alternative command that doesn't require plugins".cyan());
                 }
                 
                 println!("{} {}", "📝".cyan(), "If you know the correct command, I can learn from this for future requests.".cyan());
                
                print!("{} ", "Enter the correct command (or press Enter to skip):".green().bold());
                io::stdout().flush()?;
                
                let mut correction = String::new();
                io::stdin().read_line(&mut correction)?;
                let correction = correction.trim();
                
                if !correction.is_empty() {
                    // Store the correction for future learning
                    if let Err(e) = rag_engine.store_command_correction(user_input, command, correction).await {
                        println!("{} {}: {}", "⚠️".yellow(), "Failed to store correction".yellow(), e);
                    } else {
                        println!("{} {}", "✅".green(), "Thank you! I'll remember this correction.".green());
                    }
                }
            }
        }
        
        if !output.status.success() {
            println!("{} {}", "❌".red(), "Command failed".red());
            return Ok(false);
        } else {
            println!("{} {}", "✅".green(), "Command executed successfully".green());
            return Ok(true);
        }
    }
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
    
    // Enhanced startup with Carbon Design System principles
    display_banner();
    println!("{} {}", "💬".blue(), "Starting IBM Cloud AI chat mode...".blue());
    println!("{}", "Enhanced with watsonx.ai and Carbon Design System".italic().dimmed());
    println!();
    println!("{} {}", "📖".cyan(), "Usage Guide:".cyan().bold());
    println!("  • Type natural language queries (e.g., 'list my watson services')");
    println!("  • Use ↑/↓ arrow keys to navigate command history");
    println!("  • Press Esc to cancel current input");
    println!("  • Type 'exec <command>' to execute a command directly");
    println!("  • Type 'exit' or 'quit' to end the session");
    println!();
            
    // Enhanced initialization with better error handling
    println!("{} {}", "🔄".yellow(), "Initializing watsonx.ai connection...".yellow());
    let mut watsonx = match WatsonxAI::new() {
        Ok(w) => w,
        Err(e) => {
            println!("{} {}: {}", "❌".red(), "Failed to initialize WatsonX".red(), e);
            println!("{} {}", "💡".cyan(), "Please check your .env file and ensure WATSONX_API_KEY and WATSONX_PROJECT_ID are set".cyan());
            return Err(e);
        }
    };
    
    match watsonx.connect().await {
        Ok(_) => println!("{} {}", "✅".green(), "Connected to watsonx.ai successfully".green()),
        Err(e) => {
            println!("{} {}: {}", "❌".red(), "Failed to connect to WatsonX".red(), e);
            println!("{} {}", "💡".cyan(), "Please verify your API credentials and network connection".cyan());
            return Err(e);
        }
    }
    
    // Create local RAG engine with file-based storage
    let rag_engine = LocalRAGEngine::new(watsonx.clone(), "./rag_data.json").await?;
    
    // Create command translator with feedback capabilities
    let mut translator = CommandTranslator::new(watsonx);
    
    // Configure retry settings for better results
    let retry_config = RetryConfig {
        max_attempts: 3,
        base_timeout: std::time::Duration::from_secs(30),
        enable_progressive_prompts: true,
        quality_threshold: 0.7,
    };
    translator.set_retry_config(retry_config);
    
    // Initialize command history with Carbon-inspired UX
    let mut command_history: Vec<String> = Vec::new();
    
    println!("{} {}", "🚀".green(), "Ready! Start typing your IBM Cloud queries...".green());
    println!();
    
    // Check if we're in piped mode
    let is_piped = !io::stdin().is_terminal();
    
    // Enhanced chat loop with better error handling and user experience
    loop {
        let input = match handle_input_with_history(&mut command_history).await {
            Ok(input) => input,
            Err(e) => {
                println!("{} {}: {}", "❌".red(), "Input error".red(), e);
                if is_piped {
                    break; // Exit on error in piped mode
                }
                continue;
            }
        };
        
        if input.is_empty() {
            if is_piped {
                break; // Exit on empty input in piped mode
            }
            continue;
        }
        
        if input == "exit" || input == "quit" {
            println!("{} {}", "👋".blue(), "Thank you for using IBM Cloud AI Assistant!".blue());
            break;
        }
        
        if input.starts_with("exec ") {
            let command = input.trim_start_matches("exec ").trim();
            match execute_command_with_feedback(command, &rag_engine, &input, &mut translator).await {
                Ok(_) => {},
                Err(e) => {
                    println!("{} {}: {}", "❌".red(), "Execution error".red(), e);
                }
            }
            continue;
        }
        
        if input == "clear" {
            translator.clear_failure_history();
            println!("{} {}", "🧹".green(), "Cleared failure history for fresh start".green());
            continue;
        }
        
        // Enhanced translation with intelligent retry and feedback
        println!("{} {}", "🤔".cyan(), "Processing with watsonx.ai (with feedback learning)...".cyan());
        
        match translator.translate_with_feedback(&input).await {
            Ok(command) => {
                // Analyze command quality and provide feedback
                let quality_analysis = translator.analyze_command_quality(&command.result, &input);
                
                // Carbon Design: Clear visual hierarchy and actionable information
                println!();
                let quality_indicator = if quality_analysis.metrics.overall_score >= 0.8 {
                    "🎯".green()
                } else if quality_analysis.metrics.overall_score >= 0.6 {
                    "⚡".yellow()
                } else {
                    "⚠️".red()
                };
                
                println!("{} {} (Quality: {:.1}%)", 
                    quality_indicator, 
                    "Generated IBM Cloud CLI command:".bold(),
                    quality_analysis.metrics.overall_score * 100.0
                );
                
                // Get success rate from learning engine
                let success_rate = translator.get_command_success_rate(&command.result);
                if success_rate < 0.7 {
                    println!("{} Command success rate: {:.1}% (based on historical data)", "📊".cyan(), success_rate * 100.0);
                    let retry_suggestions = translator.get_intelligent_retry_suggestions(&command.result, &input, 0);
                    if !retry_suggestions.is_empty() {
                        println!("{} Retry suggestions:", "💡".cyan());
                        for suggestion in retry_suggestions.iter().take(3) {
                            println!("   • {}", suggestion);
                        }
                    }
                }
                
                // Responsive box sizing based on terminal width
                let terminal_width = size().map(|(w, _)| w as usize).unwrap_or(80);
                let max_box_width = terminal_width.saturating_sub(4);
                let min_box_width = 50;
                let preferred_width = std::cmp::max(command.result.len() + 4, min_box_width);
                let box_width = std::cmp::min(preferred_width, max_box_width);
                
                let top_border = format!("┌{}┐", "─".repeat(box_width - 2));
                let bottom_border = format!("└{}┘", "─".repeat(box_width - 2));
                
                println!("{}", top_border);
                
                // Extract command string from GenerationAttempt
                let command_str = &command.result;
                
                // Handle long commands with proper text wrapping
                let content_width = box_width - 4;
                if command_str.len() <= content_width {
                    println!("│ {} │", format!("{:<width$}", command_str, width = content_width));
                } else {
                    // Split long commands into multiple lines
                    let mut remaining = command_str.as_str();
                    while !remaining.is_empty() {
                        let chunk_end = if remaining.len() <= content_width {
                            remaining.len()
                        } else {
                            // Try to break at a space near the width limit
                            remaining[..content_width]
                                .rfind(' ')
                                .unwrap_or(content_width)
                        };
                        
                        let chunk = &remaining[..chunk_end];
                        println!("│ {} │", format!("{:<width$}", chunk, width = content_width));
                        remaining = &remaining[chunk_end..].trim_start();
                    }
                }
                
                println!("{}", bottom_border);
                
                // Show quality insights if score is below excellent
                if quality_analysis.metrics.overall_score < 0.8 {
                    println!();
                    println!("{} {}", "📊".cyan(), "Quality Analysis:".cyan().bold());
                    
                    if !quality_analysis.improvement_areas.is_empty() {
                        println!("{} Areas for improvement:", "🔍".yellow());
                        for area in &quality_analysis.improvement_areas {
                            println!("  • {}", area);
                        }
                    }
                    
                    if !quality_analysis.recommended_actions.is_empty() && quality_analysis.recommended_actions.len() <= 3 {
                        println!("{} Suggestions:", "💡".cyan());
                        for action in quality_analysis.recommended_actions.iter().take(3) {
                            println!("  • {}", action);
                        }
                    }
                }
                
                // Enhanced command editing with better UX
                if is_piped {
                    // In piped mode, execute the command directly without editing
                    match execute_command_with_feedback(&command.result, &rag_engine, &input, &mut translator).await {
                        Ok(success) => {
                            if success {
                                println!("{} {}", "✅".green(), "Command executed successfully!".green());
                            } else {
                                println!("{} {}", "⚠️".yellow(), "Command completed with warnings".yellow());
                            }
                        }
                        Err(e) => {
                            println!("{} {}: {}", "❌".red(), "Execution failed".red(), e);
                            println!("{} {}", "💡".cyan(), "This failure has been recorded for learning. Try rephrasing or use 'clear' to reset.".cyan());
                        }
                    }
                    break; // Exit after processing in piped mode
                } else {
                    match handle_edit_input(&command.result).await {
                        Ok(Some(final_command)) => {
                            match execute_command_with_feedback(&final_command, &rag_engine, &input, &mut translator).await {
                                Ok(success) => {
                                    if success {
                                        println!("{} {}", "✅".green(), "Command executed successfully!".green());
                                    } else {
                                        println!("{} {}", "⚠️".yellow(), "Command completed with warnings".yellow());
                                    }
                                },
                                Err(e) => {
                                    println!("{} {}: {}", "❌".red(), "Execution failed".red(), e);
                                    println!("{} {}", "💡".cyan(), "This failure has been recorded for learning. Try rephrasing or use 'clear' to reset.".cyan());
                                }
                            }
                        }
                        Ok(None) => {
                            println!("{} {}", "⏭️".yellow(), "Command execution cancelled".yellow());
                        }
                        Err(e) => {
                            println!("{} {}: {}", "❌".red(), "Edit error".red(), e);
                        }
                    }
                }
            }
            Err(e) => {
                // Enhanced error handling with actionable guidance
                println!("{} {}: {}", "❌".red(), "Translation failed".red(), e);
                
                if is_piped {
                    break; // Exit on translation error in piped mode
                }
                
                println!();
                println!("{} {}", "💡".cyan(), "Suggestions:".cyan().bold());
                println!("  • Try rephrasing your query more specifically");
                println!("  • Use IBM Cloud service names (e.g., 'watson', 'code engine')");
                println!("  • Check your network connection and API credentials");
                println!("  • Example queries:");
                println!("    - 'list my watson machine learning services'");
                println!("    - 'show code engine applications'");
                println!("    - 'login with sso'");
            }
        }
        
        println!(); // Add spacing for better readability
    }
    
    Ok(())
}