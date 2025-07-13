//! Chat session management module
//! 
//! Handles interactive chat sessions, conversation history, and terminal UI.

use crate::api::{Content, GeminiClient};
use crate::agent::Agent;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use colored::*;
use dialoguer::console;
use crossterm::event::{read, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tokio_stream::StreamExt;
use uuid::Uuid;

pub mod session;
pub mod history;
pub mod display;
pub mod agent_commands;

/// A chat session with conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    /// Unique session ID
    pub id: String,
    /// Model being used
    pub model: String,
    /// System instruction
    pub system_instruction: Option<String>,
    /// Conversation history
    pub history: Vec<Content>,
    /// Session creation time
    pub created_at: DateTime<Utc>,
    /// Last updated time
    pub updated_at: DateTime<Utc>,
}

impl ChatSession {
    /// Create a new chat session
    pub fn new(model: String, system_instruction: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            model,
            system_instruction,
            history: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Load a chat session from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let session: ChatSession = serde_json::from_str(&content)?;
        Ok(session)
    }

    /// Save the chat session to file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Add a message to the conversation history
    pub fn add_message(&mut self, content: Content) {
        self.history.push(content);
        self.updated_at = Utc::now();
    }

    /// Send a message and get response
    pub async fn send_message(&mut self, client: &GeminiClient, message: &str) -> Result<String> {
        // Add user message to history
        self.add_message(Content::user(message.to_string()));

        // Send to API
        let response = client
            .send_message(
                &self.model,
                message,
                &self.history[..self.history.len() - 1], // Don't include the message we just added
                self.system_instruction.as_deref(),
            )
            .await?;

        // Add model response to history
        self.add_message(Content::model(response.clone()));

        Ok(response)
    }

    /// Send a message with streaming response
    pub async fn send_message_stream(
        &mut self,
        client: &GeminiClient,
        message: &str,
    ) -> Result<impl tokio_stream::Stream<Item = Result<String>>> {
        // Add user message to history
        self.add_message(Content::user(message.to_string()));

        // Get streaming response
        let stream = client
            .send_message_stream(
                &self.model,
                message,
                &self.history[..self.history.len() - 1], // Don't include the message we just added
                self.system_instruction.as_deref(),
            )
            .await?;

        Ok(stream)
    }

    /// Start interactive chat mode
    pub async fn start_interactive_chat(
        &mut self,
        client: &GeminiClient,
        auto_save: bool,
    ) -> Result<()> {
        self.start_interactive_chat_with_agent(client, auto_save, None).await
    }

    /// Start interactive chat mode with optional agent support
    pub async fn start_interactive_chat_with_agent(
        &mut self,
        client: &GeminiClient,
        auto_save: bool,
        mut agent: Option<Agent>,
    ) -> Result<()> {
        // Display welcome message
        self.display_welcome();

        // Show agent status if available
        if let Some(ref agent) = agent {
            if agent.is_enabled() {
                println!("🤖 {} Agent mode is active! I can help with file operations.", "AGENT:".bright_green().bold());
                println!("   Use '/agent help' for agent commands.");
            }
        }

        // Track recent messages for completion detection
        let mut recent_messages = Vec::new();

        // Main chat loop
        loop {
            // Get user input
            print!("\n");
            let prompt = format!("{} ", "You:".bright_blue().bold());
            let input = read_input_with_features(&prompt)?;
            let input = input.trim();

            // Handle special commands
            if input.is_empty() {
                continue;
            }

            if input == "exit" || input == "quit" {
                println!("👋 Goodbye!");
                break;
            }

            if input.starts_with('/') {
                // Handle agent commands
                if input.starts_with("/agent") {
                    let parts: Vec<&str> = input.splitn(2, ' ').collect();
                    let args = parts.get(1).unwrap_or(&"");
                    if let Err(e) = agent_commands::handle_agent_command("/agent", args, &mut agent).await {
                        println!("❌ Agent command error: {}", e);
                    }
                    continue;
                }

                // Handle regular commands
                if let Err(e) = self.handle_command(input, client).await {
                    println!("❌ Command error: {}", e);
                }
                continue;
            }

            // Process agent tools if enabled
            if let Ok(Some(tool_result)) = agent_commands::process_agent_tools(input, &mut agent).await {
                // If agent tools were executed, include their results in the conversation
                let enhanced_message = format!("{}\n\nAgent tool results:\n{}", input, tool_result);
                
                // Add user message and tool results to history
                self.add_message(Content::user(enhanced_message.clone()));
                
                // Continue with AI response using the enhanced message
                let ai_input = &enhanced_message;
                
                // Show thinking indicator
                let spinner = ProgressBar::new_spinner();
                spinner.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .unwrap()
                        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
                );
                spinner.set_message("Gemini is thinking...");
                spinner.enable_steady_tick(std::time::Duration::from_millis(100));

                // Send enhanced message to AI
                match self.send_ai_response(client, ai_input, &spinner).await {
                    Ok(response) => {
                        recent_messages.push(response);
                    }
                    Err(e) => {
                        println!("❌ AI response failed: {}", e);
                        continue;
                    }
                }
            } else {
                // Regular message without agent tools
                self.add_message(Content::user(input.to_string()));
                recent_messages.push(input.to_string());

                // Show thinking indicator
                let spinner = ProgressBar::new_spinner();
                spinner.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .unwrap()
                        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
                );
                spinner.set_message("Gemini is thinking...");
                spinner.enable_steady_tick(std::time::Duration::from_millis(100));

                // Send regular message to AI
                match self.send_ai_response(client, input, &spinner).await {
                    Ok(response) => {
                        recent_messages.push(response);
                    }
                    Err(e) => {
                        println!("❌ AI response failed: {}", e);
                        continue;
                    }
                }
            }

            // Keep only recent messages for completion detection
            if recent_messages.len() > 10 {
                recent_messages.drain(0..recent_messages.len() - 10);
            }

            // Check for task completion
            if agent_commands::check_task_completion(&recent_messages, &agent) {
                println!("\n🎉 {} Task appears to be complete! The agent has finished the requested work.", "AGENT:".bright_green().bold());
                println!("   You can continue the conversation or type 'exit' to quit.");
            }

            // Auto-save if enabled
            if auto_save {
                let filename = format!("session_{}.json", self.id);
                if let Err(e) = self.save_to_file(&filename).await {
                    println!("⚠️  Failed to auto-save session: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Display welcome message
    fn display_welcome(&self) {
        println!("{}", "🤖 Chatter - Gemini AI Chat".bright_cyan().bold());
        println!("Model: {} | Session: {}", 
                 self.model.bright_yellow(), 
                 self.id[..8].bright_magenta());
        
        if let Some(ref instruction) = self.system_instruction {
            println!("System: {}", instruction.bright_white());
        }
        
        println!("{}", "─".repeat(60).bright_black());
        println!("Type 'exit' to quit, '/help' for commands");
        
        // Show conversation history if any
        if !self.history.is_empty() {
            println!("\n{}", "📜 Previous conversation:".bright_white().bold());
            for content in &self.history {
                self.display_message(content);
            }
        }
    }

    /// Display a single message
    fn display_message(&self, content: &Content) {
        let (prefix, color) = match content.role.as_str() {
            "user" => ("You:", "bright_blue"),
            "model" => ("Gemini:", "bright_green"),
            _ => ("System:", "bright_yellow"),
        };

        if let Some(part) = content.parts.first() {
            match color {
                "bright_blue" => println!("\n{} {}", prefix.bright_blue().bold(), part.text),
                "bright_green" => println!("\n{} {}", prefix.bright_green().bold(), part.text),
                _ => println!("\n{} {}", prefix.bright_yellow().bold(), part.text),
            }
        }
    }

    /// Handle special commands
    async fn handle_command(&mut self, command: &str, _client: &GeminiClient) -> Result<()> {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        let cmd = parts[0];
        let args = parts.get(1).unwrap_or(&"");

        match cmd {
            "/help" => {
                println!("📋 Available commands:");
                println!("  /help                    - Show this help");
                println!("  /clear                   - Clear conversation history");
                println!("  /save <file>             - Save session to file");
                println!("  /load <file>             - Load session from file");
                println!("  /model <name>            - Switch model");
                println!("  /system <text>           - Set system instruction");
                println!("  /template <name>         - Use template as system instruction");
                println!("  /templates               - List available templates");
                println!("  /save-template <name>    - Save current system instruction as template");
                println!("  /history                 - Show conversation history");
                println!("  /info                    - Show session info");
            }
            "/template" => {
                if args.is_empty() {
                    println!("Usage: /template <name>");
                    return Ok(());
                }
                
                // Load template manager
                let manager = crate::templates::TemplateManager::new().await?;
                if let Some(template) = manager.get(args) {
                    self.system_instruction = Some(template.content.clone());
                    println!("📝 Applied template: {} - {}", template.name.bright_green(), template.description);
                } else {
                    println!("❌ Template '{}' not found", args);
                }
            }
            "/templates" => {
                // Load template manager and list templates
                let manager = crate::templates::TemplateManager::new().await?;
                let templates = manager.list_all();
                
                if templates.is_empty() {
                    println!("📭 No templates available");
                    return Ok(());
                }
                
                println!("📋 Available Templates:");
                
                // Group by category
                let mut by_category: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
                for template in templates {
                    by_category.entry(template.category.clone()).or_default().push(template);
                }
                
                for (cat, templates) in by_category {
                    println!("\n{}", cat.bright_cyan().bold());
                    for template in templates {
                        let builtin_marker = if template.builtin { " (built-in)".bright_black() } else { "".normal() };
                        println!("  {} - {}{}", 
                                 template.name.bright_green(), 
                                 template.description,
                                 builtin_marker);
                    }
                }
                println!();
            }
            "/clear" => {
                self.history.clear();
                println!("🗑️  Conversation history cleared");
            }
            "/save" => {
                if args.is_empty() {
                    return Err(anyhow!("Please specify a filename"));
                }
                self.save_to_file(args).await?;
                println!("💾 Session saved to {}", args);
            }
            "/model" => {
                if args.is_empty() {
                    println!("Current model: {}", self.model);
                } else {
                    self.model = args.to_string();
                    println!("🔄 Switched to model: {}", self.model);
                }
            }
            "/system" => {
                if args.is_empty() {
                    match &self.system_instruction {
                        Some(instruction) => println!("Current system instruction: {}", instruction),
                        None => println!("No system instruction set"),
                    }
                } else {
                    self.system_instruction = Some(args.to_string());
                    println!("⚙️  System instruction updated");
                }
            }
            "/history" => {
                if self.history.is_empty() {
                    println!("📭 No conversation history");
                } else {
                    println!("📜 Conversation history ({} messages):", self.history.len());
                    for content in &self.history {
                        self.display_message(content);
                    }
                }
            }
            "/save-template" => {
                if args.is_empty() {
                    println!("Usage: /save-template <name>");
                    return Ok(());
                }
                
                // Check if we have a system instruction to save
                if let Some(ref instruction) = self.system_instruction {
                    use dialoguer::Input;
                    
                    // Get template details interactively
                    let description: String = Input::new()
                        .with_prompt("Template description")
                        .interact()?;
                    
                    let category: String = Input::new()
                        .with_prompt("Template category")
                        .default("custom".to_string())
                        .interact()?;
                    
                    let tags_input: String = Input::new()
                        .with_prompt("Tags (comma-separated)")
                        .default("".to_string())
                        .interact()?;
                    
                    let tags: Vec<String> = tags_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    
                    // Create and save template
                    let template = crate::templates::Template::new(
                        args.to_string(),
                        description,
                        instruction.clone(),
                        category,
                        tags,
                    );
                    
                    let mut manager = crate::templates::TemplateManager::new().await?;
                    match manager.create(template).await {
                        Ok(()) => {
                            println!("✅ Template '{}' saved successfully!", args);
                        }
                        Err(e) => {
                            println!("❌ Failed to save template: {}", e);
                        }
                    }
                } else {
                    println!("❌ No system instruction set. Use /system <text> first.");
                }
            }
            "/info" => {
                println!("📊 Session Information:");
                println!("  ID: {}", self.id);
                println!("  Model: {}", self.model);
                println!("  Messages: {}", self.history.len());
                println!("  Created: {}", self.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
                println!("  Updated: {}", self.updated_at.format("%Y-%m-%d %H:%M:%S UTC"));
            }
            _ => {
                return Err(anyhow!("Unknown command: {}. Type /help for available commands", cmd));
            }
        }

        Ok(())
    }

    /// Send a message to AI and handle the response with streaming
    async fn send_ai_response(
        &mut self,
        client: &GeminiClient,
        message: &str,
        spinner: &ProgressBar,
    ) -> Result<String> {
        // Send message with streaming, fallback to non-streaming on failure
        match self.send_message_stream(client, message).await {
            Ok(mut stream) => {
                spinner.finish_and_clear();
                print!("\n{} ", "Gemini:".bright_green().bold());
                io::stdout().flush()?;

                let mut full_response = String::new();
                let mut stream_failed = false;
                
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            print!("{}", chunk);
                            io::stdout().flush()?;
                            full_response.push_str(&chunk);
                        }
                        Err(e) => {
                            println!("\n⚠️  Stream error: {}", e);
                            println!("🔄 Falling back to non-streaming mode...");
                            stream_failed = true;
                            break;
                        }
                    }
                }

                // If streaming failed, try non-streaming mode
                if stream_failed {
                    match self.send_message(client, message).await {
                        Ok(response) => {
                            println!("\n{} {}", "Gemini:".bright_green().bold(), response);
                            full_response = response;
                        }
                        Err(e) => {
                            return Err(anyhow!("Non-streaming fallback also failed: {}", e));
                        }
                    }
                } else {
                    // Add the complete response to history
                    if !full_response.is_empty() {
                        self.add_message(Content::model(full_response.clone()));
                    }
                    println!(); // New line after response
                }

                Ok(full_response)
            }
            Err(e) => {
                spinner.finish_and_clear();
                println!("⚠️  Streaming failed: {}", e);
                println!("🔄 Trying non-streaming mode...");
                
                // Fallback to non-streaming mode
                match self.send_message(client, message).await {
                    Ok(response) => {
                        println!("\n{} {}", "Gemini:".bright_green().bold(), response);
                        Ok(response)
                    }
                    Err(e) => {
                        Err(anyhow!("Non-streaming fallback also failed: {}", e))
                    }
                }
            }
        }
    }
}

/// Read user input with support for arrow keys, backspace, and multiline input.
fn read_input_with_features(prompt: &str) -> Result<String> {
    enable_raw_mode()?;
    let mut buffer = String::new();
    let mut position = 0;

    // Print prompt for the first time
    print!("{}", prompt);
    io::stdout().flush()?;

    let prompt_width = console::strip_ansi_codes(prompt).chars().count();

    loop {
        if let Event::Key(KeyEvent { code, modifiers, .. }) = read()? {
            match code {
                KeyCode::Char(c) => {
                    if modifiers == KeyModifiers::SHIFT && c == 'M' {
                        // This is likely a Shift+Enter paste, handle it as a newline
                        buffer.insert(position, '\n');
                        position += 1;
                    } else {
                        buffer.insert(position, c);
                        position += 1;
                    }
                }
                KeyCode::Backspace => {
                    if position > 0 {
                        position -= 1;
                        buffer.remove(position);
                    }
                }
                KeyCode::Enter => {
                    if modifiers == KeyModifiers::SHIFT {
                        buffer.insert(position, '\n');
                        position += 1;
                    } else {
                        break;
                    }
                }
                KeyCode::Left => {
                    if position > 0 {
                        position -= 1;
                    }
                }
                KeyCode::Right => {
                    if position < buffer.len() {
                        position += 1;
                    }
                }
                _ => {}
            }

            // Redraw the line
            print!("\r\x1B[K"); // Clear the line
            print!("{}", prompt); // Reprint the prompt
            print!("{}", buffer);
            print!("\r\x1B[{}C", prompt_width + position); // Move cursor to position
            io::stdout().flush()?;
        }
    }

    disable_raw_mode()?;
    Ok(buffer)
}
