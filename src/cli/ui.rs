//! UI utilities for the CLI

use colored::*;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, size},
};
use std::io::{self, Write, IsTerminal};
use std::process::Command;
use crate::core::{Result, CloudProviderType};
use super::CommandLearningEngine;
use anyrepair::Repair;

/// Display startup banner with Carbon Design System inspired styling
pub fn display_banner() {
    let terminal_width = size().map(|(w, _)| w as usize).unwrap_or(80);
    let banner_width = std::cmp::min(67, terminal_width.saturating_sub(4));

    let top_border = format!("┌{}┐", "─".repeat(banner_width - 2));
    let bottom_border = format!("└{}┘", "─".repeat(banner_width - 2));
    let empty_line = format!("│{}│", " ".repeat(banner_width - 2));

    println!();
    println!("{}", top_border.blue());
    println!("{}", empty_line.blue());

    let title_line = format!(
        "│  {}{}│",
        "AnyCLI - Cloud Universal CLI".blue().bold(),
        " ".repeat(banner_width - 31)
    );
    println!("{}", title_line);

    println!("{}", empty_line.blue());

    let feature_lines = vec![
        "🤖 AI-Powered Universal CLI Assistant",
        "",
        "Features:",
        "• 🚀 Natural language to cloud commands",
        "• 🔧 Intelligent error handling & suggestions",
        "• 📝 Interactive command editing (Esc to cancel)",
        "• ⬆️  Command history navigation (↑/↓ arrows)",
        "• 🔐 Automatic login status verification",
        "",
        "v0.1.0 • Powered by watsonx.ai",
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
    println!(
        "{}",
        "💡 Tip: Type your request in natural language, or 'help' for commands".dimmed()
    );
    println!();
}

/// Handle input with command history navigation
pub async fn handle_input_with_history(history: &mut Vec<String>) -> Result<String> {
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

    print!("{} ", "anycli>".green().bold());
    io::stdout().flush()?;

    loop {
        if let Event::Key(key_event) = event::read()? {
            match key_event.code {
                KeyCode::Enter => {
                    disable_raw_mode()?;
                    println!();
                    if !input.is_empty() {
                        history.push(input.clone());
                    }
                    return Ok(input);
                }
                KeyCode::Char(c) => {
                    input.insert(cursor_pos, c);
                    cursor_pos += 1;
                    print!("\r{} {}", "anycli>".green().bold(), input);
                    io::stdout().flush()?;
                }
                KeyCode::Backspace => {
                    if cursor_pos > 0 {
                        input.remove(cursor_pos - 1);
                        cursor_pos -= 1;
                        print!("\r{} {}  \r{} {}", "anycli>".green().bold(), input, "anycli>".green().bold(), input);
                        io::stdout().flush()?;
                    }
                }
                KeyCode::Up => {
                    if !history.is_empty() {
                        let new_index = match history_index {
                            None => history.len() - 1,
                            Some(idx) if idx > 0 => idx - 1,
                            Some(idx) => idx,
                        };
                        history_index = Some(new_index);
                        input = history[new_index].clone();
                        cursor_pos = input.len();
                        print!("\r{} {}  \r{} {}", "anycli>".green().bold(), " ".repeat(50), "anycli>".green().bold(), input);
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
                        print!("\r{} {}  \r{} {}", "anycli>".green().bold(), " ".repeat(50), "anycli>".green().bold(), input);
                        io::stdout().flush()?;
                    }
                }
                KeyCode::Esc => {
                    disable_raw_mode()?;
                    println!();
                    return Ok(String::new());
                }
                _ => {}
            }
        }
    }
}

/// Display help message
pub fn print_help() {
    println!("{}", "Available commands:".bold());
    println!("  {} - Type natural language queries to translate to cloud commands", "query".green());
    println!("  {} - Deploy applications to cloud (e.g., 'deploy to code engine')", "deploy".green());
    println!("  {} - Execute a command directly", "exec <command>".green());
    println!("  {} - Show this help message", "help".green());
    println!("  {} - Exit the application", "exit/quit".green());
    println!();
    println!("{}", "Examples:".bold());
    println!("  list my resource groups");
    println!("  show all kubernetes clusters");
    println!("  deploy my app to code engine");
    println!("  deploy app named myapp to project myproject");
    println!("  exec ibmcloud target --cf");
}

/// Confirm command execution with user
pub async fn confirm_execution(_command: &str) -> Result<bool> {
    print!("{} Execute this command? [Y/n]: ", "❓".cyan());
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;
    let response = response.trim().to_lowercase();

    Ok(response.is_empty() || response == "y" || response == "yes")
}

/// Result of command execution
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Execute a shell command and return detailed result
pub async fn execute_command(command: &str) -> Result<CommandResult> {
    execute_command_with_provider(command, None).await
}

/// Execute a shell command with provider-aware JSON repair
pub async fn execute_command_with_provider(
    command: &str,
    provider: Option<CloudProviderType>,
) -> Result<CommandResult> {
    // Check login status for IBM Cloud commands before executing
    if let Some(p) = provider {
        if p == CloudProviderType::IBMCloud && command.starts_with("ibmcloud") && !command.contains("login") {
            if let Err(e) = ensure_ibmcloud_login().await {
                println!("{} {}: {}", "⚠️".yellow(), "Login required".yellow(), e);
                println!("{}", "Please run 'ibmcloud login' first".cyan());
                return Ok(CommandResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("Not logged in to IBM Cloud: {}", e),
                });
            }
        }
    }

    println!("{} Executing...", "🚀".yellow());

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", command]).output()?
    } else {
        Command::new("sh").arg("-c").arg(command).output()?
    };

    let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Repair JSON output for AWS commands if needed
    if let Some(p) = provider {
        if p == CloudProviderType::AWS && command.contains("--output json") && !stdout.is_empty() {
            stdout = repair_aws_json_output(&stdout)?;
        }
    }

    if !stdout.is_empty() {
        println!("{}", stdout);
    }

    if !stderr.is_empty() {
        eprintln!("{}", stderr.red());
    }

    let success = output.status.success();
    if success {
        println!("{} Command executed successfully", "✅".green());
    } else {
        println!("{} Command failed", "❌".red());
    }

    Ok(CommandResult {
        success,
        stdout,
        stderr,
    })
}

/// Check if IBM Cloud CLI is logged in
pub async fn check_ibmcloud_login() -> Result<bool> {
    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", "ibmcloud account show"])
            .output()?
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("ibmcloud account show")
            .output()?
    };
    
    Ok(output.status.success())
}

/// Ensure user is logged in to IBM Cloud
pub async fn ensure_ibmcloud_login() -> Result<()> {
    if !check_ibmcloud_login().await? {
        println!("{}", "🔐 IBM Cloud login required".yellow());
        println!("{}", "Please run: ibmcloud login".cyan());
        return Err(crate::core::Error::Authentication(
            "Not logged in to IBM Cloud".to_string()
        ));
    }
    Ok(())
}

/// Repair malformed JSON output from AWS CLI commands using anyrepair
fn repair_aws_json_output(output: &str) -> Result<String> {
    // Try to extract JSON from the output
    let lines: Vec<&str> = output.lines().collect();
    let mut json_lines = Vec::new();
    let mut in_json = false;
    
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            in_json = true;
        }
        if in_json {
            json_lines.push(line);
            if trimmed.ends_with('}') || trimmed.ends_with(']') {
                break;
            }
        }
    }
    
    if json_lines.is_empty() {
        return Ok(output.to_string());
    }
    
    let json_text = json_lines.join("\n");
    
    // Try to parse as JSON first
    if serde_json::from_str::<serde_json::Value>(&json_text).is_ok() {
        return Ok(output.to_string());
    }
    
    // Use anyrepair to repair the JSON
    match anyrepair::json::JsonRepairer::new().repair(&json_text) {
        Ok(repaired) => {
            let repaired = repaired.to_string();
            // Validate that it's valid JSON
            if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
                println!("{}", "🔧 JSON repaired successfully".green());
                // Replace the original JSON section with repaired version
                let mut result = output.to_string();
                if let Some(json_start) = result.find('{') {
                    if let Some(json_end) = result.rfind('}') {
                        let before = &result[..json_start];
                        let after = &result[json_end + 1..];
                        result = format!("{}{}{}", before, repaired, after);
                    }
                }
                Ok(result)
            } else {
                Ok(output.to_string())
            }
        }
        Err(_) => Ok(output.to_string()),
    }
}

/// Handle learning from failed commands
pub async fn handle_learning(
    query: &str,
    failed_command: &str,
    learning_engine: &mut CommandLearningEngine,
) -> Result<()> {
    println!("{} Would you like to provide the correct command?", "📝".cyan());
    print!("Correct command (or press Enter to skip): ");
    io::stdout().flush()?;

    let mut correction = String::new();
    io::stdin().read_line(&mut correction)?;
    let correction = correction.trim();

    if !correction.is_empty() {
        learning_engine.add_correction(
            query.to_string(),
            correction.to_string(),
            Some(format!("Failed command: {}", failed_command)),
        ).await?;
        println!("{} Thanks! I'll remember this.", "✅".green());
    }

    Ok(())
}
