//! Chat session management module
//!
//! Handles interactive chat sessions, conversation history, and terminal UI.

use crate::agent::{Agent, ToolCall, ToolResult};
use crate::api::{Content, LlmClient, ModelToolCall, Part};
use crate::config::ModelProvider;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tokio_stream::StreamExt;
use uuid::Uuid;

pub mod agent_commands;
pub mod display;
pub mod history;
pub mod session;

use agent_commands::format_tool_result;
/// A chat session with conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    /// Unique session ID
    pub id: String,
    /// Model being used
    pub model: String,
    /// Model provider
    #[serde(default = "default_session_provider")]
    pub provider: ModelProvider,
    /// System instruction
    pub system_instruction: Option<String>,
    /// Conversation history
    pub history: Vec<Content>,
    /// Session creation time
    pub created_at: DateTime<Utc>,
    /// Last updated time
    pub updated_at: DateTime<Utc>,
}

fn default_session_provider() -> ModelProvider {
    ModelProvider::Gemini
}

#[derive(Debug, Clone)]
struct ToolExecutionRecord {
    tool_name: String,
    result: ToolResult,
}

#[derive(Debug, Clone)]
struct InteractionResult {
    response_text: String,
    tool_executions: Vec<ToolExecutionRecord>,
}

const MAX_TOOL_ITERATIONS: usize = 6;

impl ChatSession {
    /// Create a new chat session
    pub fn new(model: String, provider: ModelProvider, system_instruction: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            model,
            provider,
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

    async fn run_model_interaction(
        &mut self,
        client: &LlmClient,
        mut agent: Option<&mut Agent>,
    ) -> Result<InteractionResult> {
        let mut tool_executions = Vec::new();
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > MAX_TOOL_ITERATIONS {
                return Err(anyhow!(
                    "Exceeded maximum tool interaction depth ({} iterations)",
                    MAX_TOOL_ITERATIONS
                ));
            }

            let tool_definitions = if matches!(self.provider, ModelProvider::Ollama) {
                if let Some(agent_ref) = agent.as_mut() {
                    if agent_ref.is_enabled() {
                        agent_ref.tool_definitions()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            let chat_response = client
                .generate(
                    &self.model,
                    &self.history,
                    self.system_instruction.as_deref(),
                    &tool_definitions,
                )
                .await?;

            let mut assistant_message = chat_response.message;

            if assistant_message.parts.is_empty() {
                assistant_message.parts.push(Part {
                    text: String::new(),
                });
            }

            let response_text = assistant_message
                .parts
                .first()
                .map(|p| p.text.clone())
                .unwrap_or_default();

            let tool_calls = assistant_message.tool_calls.clone();

            self.add_message(assistant_message);

            if tool_calls.is_empty() {
                return Ok(InteractionResult {
                    response_text,
                    tool_executions,
                });
            }

            if !matches!(self.provider, ModelProvider::Ollama) {
                return Err(anyhow!(
                    "Received tool call from unsupported provider: {:?}",
                    self.provider
                ));
            }

            let agent_ref = agent
                .as_mut()
                .ok_or_else(|| anyhow!("Model requested tools but agent mode is not available"))?;

            if !agent_ref.is_enabled() {
                return Err(anyhow!(
                    "Model requested tools but agent mode is currently disabled"
                ));
            }

            for call in tool_calls {
                let tool_call = convert_model_tool_call(&call)?;
                let tool_name = tool_call.tool.clone();
                let call_id = call.id.clone();

                let execution_result = match agent_ref.execute_tool(tool_call.clone()).await {
                    Ok(result) => result,
                    Err(e) => ToolResult::error(format!("Tool execution error: {e}")),
                };

                let payload_json = build_tool_result_payload(&tool_name, &execution_result);
                let payload_string = serde_json::to_string(&payload_json)
                    .context("Failed to encode tool result payload")?;

                let tool_message = Content {
                    role: "tool".to_string(),
                    parts: vec![Part {
                        text: payload_string.clone(),
                    }],
                    name: Some(tool_name.clone()),
                    tool_call_id: call_id.clone(),
                    tool_calls: Vec::new(),
                };
                self.add_message(tool_message);

                tool_executions.push(ToolExecutionRecord {
                    tool_name,
                    result: execution_result,
                });
            }

            // Loop to let the model incorporate tool outputs
        }
    }

    /// Start interactive chat mode
    pub async fn start_interactive_chat(
        &mut self,
        client: &LlmClient,
        auto_save: bool,
        sessions_dir: Option<PathBuf>,
    ) -> Result<()> {
        self.start_interactive_chat_with_agent(client, auto_save, sessions_dir, None)
            .await
    }

    /// Start interactive chat mode with optional agent support
    pub async fn start_interactive_chat_with_agent(
        &mut self,
        client: &LlmClient,
        auto_save: bool,
        sessions_dir: Option<PathBuf>,
        mut agent: Option<Agent>,
    ) -> Result<()> {
        // Display welcome message
        self.display_welcome();

        // Show agent status if available
        if let Some(ref agent) = agent {
            if agent.is_enabled() {
                println!(
                    "🤖 {} Agent mode is active! I can help with file operations.",
                    "AGENT:".bright_green().bold()
                );
                println!("   Use '/agent help' for agent commands.");
            }
        }

        // Track recent messages for completion detection
        let mut recent_messages = Vec::new();

        // Main chat loop
        loop {
            // Get user input
            let prompt = format!(
                "
{} ",
                "You:".bright_blue().bold()
            );
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
                    if let Err(e) =
                        agent_commands::handle_agent_command("/agent", args, &mut agent).await
                    {
                        println!("❌ Agent command error: {e}");
                    }
                    continue;
                }

                // Handle regular commands
                if let Err(e) = self.handle_command(input).await {
                    println!("❌ Command error: {e}");
                }
                continue;
            }

            // Process agent tools if enabled
            if let Ok(Some(tool_result)) =
                agent_commands::process_agent_tools(input, &mut agent).await
            {
                // If agent tools were executed, include their results in the conversation
                let enhanced_message = format!("{input}\n\nAgent tool results:\n{tool_result}");

                // Add user message and tool results to history
                self.add_message(Content::user(enhanced_message.clone()));

                // Continue with AI response using the enhanced message
                // Show thinking indicator
                let spinner = ProgressBar::new_spinner();
                spinner.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.green} {msg}")
                        .unwrap()
                        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
                );
                spinner.set_message(format!("{} is thinking...", self.model_label()));
                spinner.enable_steady_tick(std::time::Duration::from_millis(100));

                // Send enhanced message to AI
                match self
                    .send_ai_response(client, &spinner, agent.as_mut())
                    .await
                {
                    Ok(response) => {
                        recent_messages.push(response);
                    }
                    Err(e) => {
                        println!("❌ AI response failed: {e}");
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
                spinner.set_message(format!("{} is thinking...", self.model_label()));
                spinner.enable_steady_tick(std::time::Duration::from_millis(100));

                // Send regular message to AI
                match self
                    .send_ai_response(client, &spinner, agent.as_mut())
                    .await
                {
                    Ok(response) => {
                        recent_messages.push(response);
                    }
                    Err(e) => {
                        println!("❌ AI response failed: {e}");
                        continue;
                    }
                }
            }

            // Keep only recent messages for completion detection
            if recent_messages.len() > 10 {
                recent_messages.drain(0..recent_messages.len() - 10);
            }

            // Check for task completion
            if let Some((status, confidence, patterns)) =
                agent_commands::check_task_completion(&recent_messages, &agent)
            {
                println!(
                    "\n🎉 {} Task appears to be complete! The agent has finished the requested work.",
                    "AGENT:".bright_green().bold()
                );
                println!("   {}", status.description());
                println!("   Confidence: {:.0}%", confidence * 100.0);
                if !patterns.is_empty() {
                    println!("   Matching patterns:");
                    for pattern in patterns {
                        println!("      • {}", pattern);
                    }
                }
                println!("   You can continue the conversation or type 'exit' to quit.");
            }

            // Auto-save if enabled
            if auto_save {
                let filename = format!("session_{}.json", self.id);
                let path = if let Some(ref dir) = sessions_dir {
                    if let Err(e) = fs::create_dir_all(dir) {
                        println!("⚠️  Failed to ensure sessions directory exists: {e}");
                    }
                    dir.join(filename)
                } else {
                    PathBuf::from(&filename)
                };

                if let Err(e) = self.save_to_file(&path).await {
                    println!("⚠️  Failed to auto-save session: {e}");
                }
            }
        }

        Ok(())
    }

    /// Display welcome message
    fn display_welcome(&self) {
        println!(
            "{}",
            format!("🤖 Chatter - {} AI Chat", self.model_label())
                .bright_cyan()
                .bold()
        );
        println!(
            "Model: {} | Provider: {} | Session: {}",
            self.model.bright_yellow(),
            self.model_label().bright_cyan(),
            self.id[..8].bright_magenta()
        );

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
    async fn handle_command(&mut self, command: &str) -> Result<()> {
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
                println!(
                    "  /save-template <name>    - Save current system instruction as template"
                );
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
                    println!(
                        "📝 Applied template: {} - {}",
                        template.name.bright_green(),
                        template.description
                    );
                } else {
                    println!("❌ Template '{args}' not found");
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
                let mut by_category: std::collections::HashMap<String, Vec<_>> =
                    std::collections::HashMap::new();
                for template in templates {
                    by_category
                        .entry(template.category.clone())
                        .or_default()
                        .push(template);
                }

                for (cat, templates) in by_category {
                    println!("\n{}", cat.bright_cyan().bold());
                    for template in templates {
                        let builtin_marker = if template.builtin {
                            " (built-in)".bright_black()
                        } else {
                            "".normal()
                        };
                        println!(
                            "  {} - {}{}",
                            template.name.bright_green(),
                            template.description,
                            builtin_marker
                        );
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
                println!("💾 Session saved to {args}");
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
                        Some(instruction) => println!("Current system instruction: {instruction}"),
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
                    // Get template details interactively
                    let description: String = dialoguer::Input::new()
                        .with_prompt("Template description")
                        .interact()
                        .unwrap_or_else(|_| String::new());

                    let category: String = dialoguer::Input::new()
                        .with_prompt("Template category")
                        .default("custom".to_string())
                        .interact()
                        .unwrap_or_else(|_| String::from("custom"));

                    let tags_input: String = dialoguer::Input::new()
                        .with_prompt("Tags (comma-separated)")
                        .default("".to_string())
                        .interact()
                        .unwrap_or_else(|_| String::new());

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
                            println!("✅ Template '{args}' saved successfully!");
                        }
                        Err(e) => {
                            println!("❌ Failed to save template: {e}");
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
                println!(
                    "  Created: {}",
                    self.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
                println!(
                    "  Updated: {}",
                    self.updated_at.format("%Y-%m-%d %H:%M:%S UTC")
                );
            }
            _ => {
                return Err(anyhow!(
                    "Unknown command: {}. Type /help for available commands",
                    cmd
                ));
            }
        }

        Ok(())
    }

    /// Send a message to AI and handle the response with streaming
    async fn send_ai_response(
        &mut self,
        client: &LlmClient,
        spinner: &ProgressBar,
        agent: Option<&mut Agent>,
    ) -> Result<String> {
        match self.provider {
            ModelProvider::Gemini => {
                // Streaming path for Gemini
                match client
                    .generate_stream(
                        &self.model,
                        &self.history,
                        self.system_instruction.as_deref(),
                    )
                    .await
                {
                    Ok(mut stream) => {
                        spinner.finish_and_clear();
                        print!("\n{} ", self.model_label().bright_green().bold());
                        io::stdout().flush()?;

                        let mut full_response = String::new();
                        let mut stream_failed = false;

                        while let Some(chunk_result) = stream.next().await {
                            match chunk_result {
                                Ok(chunk) => {
                                    print!("{chunk}");
                                    io::stdout().flush()?;
                                    full_response.push_str(&chunk);
                                }
                                Err(e) => {
                                    println!("\n⚠️  Stream error: {e}");
                                    println!("🔄 Falling back to non-streaming mode...");
                                    stream_failed = true;
                                    break;
                                }
                            }
                        }

                        if stream_failed {
                            let interaction = self.run_model_interaction(client, agent).await?;
                            println!(
                                "\n{} {}",
                                self.model_label().bright_green().bold(),
                                interaction.response_text
                            );
                            Ok(interaction.response_text)
                        } else {
                            if !full_response.is_empty() {
                                self.add_message(Content::model(full_response.clone()));
                            }
                            println!();
                            Ok(full_response)
                        }
                    }
                    Err(e) => {
                        spinner.finish_and_clear();
                        println!("⚠️  Streaming failed: {e}");
                        println!("🔄 Trying non-streaming mode...");
                        let interaction = self.run_model_interaction(client, agent).await?;
                        println!(
                            "\n{} {}",
                            self.model_label().bright_green().bold(),
                            interaction.response_text
                        );
                        Ok(interaction.response_text)
                    }
                }
            }
            ModelProvider::Ollama => {
                spinner.finish_and_clear();
                let interaction = self.run_model_interaction(client, agent).await?;

                for record in &interaction.tool_executions {
                    let summary = format_tool_result(&record.tool_name, &record.result);
                    println!("\n🔧 {} {}", "TOOL".bright_green().bold(), summary);
                }

                if !interaction.response_text.is_empty() {
                    println!(
                        "\n{} {}",
                        self.model_label().bright_green().bold(),
                        interaction.response_text
                    );
                }

                Ok(interaction.response_text)
            }
        }
    }

    fn model_label(&self) -> &'static str {
        match self.provider {
            ModelProvider::Gemini => "Gemini",
            ModelProvider::Ollama => "Ollama",
        }
    }

    /// Convenience helper for one-shot requests without agent tooling
    pub async fn send_with_client(&mut self, client: &LlmClient, message: &str) -> Result<String> {
        self.add_message(Content::user(message.to_string()));
        let result = self.run_model_interaction(client, None).await?;
        Ok(result.response_text)
    }
}

fn convert_model_tool_call(call: &ModelToolCall) -> Result<ToolCall> {
    let parameters = extract_argument_map(&call.arguments)?;

    Ok(ToolCall {
        tool: call.name.clone(),
        parameters,
        thought: None,
        reasoning: None,
    })
}

fn extract_argument_map(value: &Value) -> Result<HashMap<String, Value>> {
    match value {
        Value::Null => Ok(HashMap::new()),
        Value::Object(map) => Ok(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        Value::String(s) => {
            if s.trim().is_empty() {
                Ok(HashMap::new())
            } else {
                let parsed: Value = serde_json::from_str(s)
                    .context("Failed to parse tool arguments JSON string")?;
                extract_argument_map(&parsed)
            }
        }
        other => Err(anyhow!(
            "Tool arguments must be an object; received {}",
            other
        )),
    }
}

fn build_tool_result_payload(tool_name: &str, result: &ToolResult) -> Value {
    let modified_files: Vec<Value> = result
        .modified_files
        .iter()
        .map(|path| Value::String(path.display().to_string()))
        .collect();

    serde_json::json!({
        "tool": tool_name,
        "success": result.success,
        "message": result.message,
        "data": result.data.clone(),
        "modified_files": modified_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_tool_result() -> ToolResult {
        ToolResult {
            success: true,
            data: serde_json::json!({"answer": 42}),
            message: Some("All good".to_string()),
            modified_files: vec![PathBuf::from("foo.txt"), PathBuf::from("bar/baz.rs")],
        }
    }

    #[test]
    fn convert_model_tool_call_extracts_parameters() {
        let call = ModelToolCall {
            id: Some("tool-1".to_string()),
            name: "search_files".to_string(),
            arguments: serde_json::json!({
                "pattern": "TODO",
                "path": "src",
            }),
        };

        let converted = convert_model_tool_call(&call).expect("conversion should succeed");
        assert_eq!(converted.tool, "search_files");
        assert_eq!(
            converted.parameters.get("pattern").unwrap(),
            &serde_json::json!("TODO")
        );
        assert_eq!(
            converted.parameters.get("path").unwrap(),
            &serde_json::json!("src")
        );
    }

    #[test]
    fn extract_argument_map_parses_json_strings() {
        let raw =
            serde_json::Value::String("{\"path\": \"src\", \"pattern\": \"TODO\"}".to_string());
        let map = extract_argument_map(&raw).expect("stringified JSON should parse");
        assert_eq!(map.get("path").unwrap(), &serde_json::json!("src"));
        assert_eq!(map.get("pattern").unwrap(), &serde_json::json!("TODO"));
    }

    #[test]
    fn build_tool_result_payload_contains_expected_fields() {
        let payload = build_tool_result_payload("read_file", &sample_tool_result());
        assert_eq!(payload["tool"], "read_file");
        assert_eq!(payload["success"], true);
        assert_eq!(payload["message"], "All good");
        assert_eq!(payload["data"], serde_json::json!({"answer": 42}));

        let modified = payload["modified_files"].as_array().expect("array");
        assert_eq!(modified.len(), 2);
        assert!(modified.iter().any(|v| v == "foo.txt"));
        assert!(modified.iter().any(|v| v == "bar/baz.rs"));
    }
}
/// Read user input with support for arrow keys, backspace, and multiline input.
fn read_input_with_features(prompt: &str) -> Result<String> {
    let mut rl = DefaultEditor::new()?;

    let history_path = dirs::data_dir()
        .ok_or_else(|| anyhow!("Failed to find data directory"))?
        .join("chatter/history.txt");

    if let Some(parent) = history_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let _ = rl.load_history(&history_path);

    let input = match rl.readline(prompt) {
        Ok(line) => {
            let _ = rl.add_history_entry(line.as_str());
            let _ = rl.save_history(&history_path);
            Ok(line)
        }
        Err(ReadlineError::Interrupted) => {
            println!("👋 Goodbye!");
            std::process::exit(0);
        }
        Err(ReadlineError::Eof) => {
            println!("👋 Goodbye!");
            std::process::exit(0);
        }
        Err(err) => Err(anyhow!("Failed to read line: {}", err)),
    };

    input
}
