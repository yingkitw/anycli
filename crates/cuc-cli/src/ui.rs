//! UI utilities for the CLI

use colored::*;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, size},
};
use std::io::{self, Write, IsTerminal};
use std::process::Command;
use cuc_core::Result;
use crate::CommandLearningEngine;

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
        "CUC - Cloud Universal CLI".blue().bold(),
        " ".repeat(banner_width - 28)
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

    print!("{} ", "cuc>".green().bold());
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
                    print!("\r{} {}", "cuc>".green().bold(), input);
                    io::stdout().flush()?;
                }
                KeyCode::Backspace => {
                    if cursor_pos > 0 {
                        input.remove(cursor_pos - 1);
                        cursor_pos -= 1;
                        print!("\r{} {}  \r{} {}", "cuc>".green().bold(), input, "cuc>".green().bold(), input);
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
                        print!("\r{} {}  \r{} {}", "cuc>".green().bold(), " ".repeat(50), "cuc>".green().bold(), input);
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
                        print!("\r{} {}  \r{} {}", "cuc>".green().bold(), " ".repeat(50), "cuc>".green().bold(), input);
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
    println!("  {} - Execute a command directly", "exec <command>".green());
    println!("  {} - Show this help message", "help".green());
    println!("  {} - Exit the application", "exit/quit".green());
    println!();
    println!("{}", "Examples:".bold());
    println!("  list my resource groups");
    println!("  show all kubernetes clusters");
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

/// Execute a shell command and return success status
pub async fn execute_command(command: &str) -> Result<bool> {
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
