//! UI utilities for the CLI

use colored::*;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, size},
};
use std::io::{self, Write, IsTerminal};
use cuc_core::Result;

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
        "│  {}  {}{}│",
        "IBM Cloud".blue().bold(),
        "AI CLI".green().bold(),
        " ".repeat(banner_width - 20)
    );
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

    print!("{} ", "ibmcloud-ai>".green().bold());
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
                    print!("\r{} {}", "ibmcloud-ai>".green().bold(), input);
                    io::stdout().flush()?;
                }
                KeyCode::Backspace => {
                    if cursor_pos > 0 {
                        input.remove(cursor_pos - 1);
                        cursor_pos -= 1;
                        print!("\r{} {}  \r{} {}", "ibmcloud-ai>".green().bold(), input, "ibmcloud-ai>".green().bold(), input);
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
                        print!("\r{} {}  \r{} {}", "ibmcloud-ai>".green().bold(), " ".repeat(50), "ibmcloud-ai>".green().bold(), input);
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
                        print!("\r{} {}  \r{} {}", "ibmcloud-ai>".green().bold(), " ".repeat(50), "ibmcloud-ai>".green().bold(), input);
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
