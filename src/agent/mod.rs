//! Agent module for autonomous task execution
//!
//! Provides tools for file operations, search, and autonomous task completion
//! within a safe, sandboxed environment.

use crate::api::ToolDefinition;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub mod completion;
pub mod executor;
pub mod safety;
pub mod tools;

pub use completion::{CompletionDetector, CompletionStatus};
pub use executor::AgentExecutor;
pub use safety::SafetyManager;
pub use tools::{ToolCall, ToolResult};

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Whether agent mode is enabled
    pub enabled: bool,
    /// Allowed file extensions for operations
    pub allowed_extensions: Vec<String>,
    /// Maximum file size in bytes
    pub max_file_size: usize,
    /// Working directory for operations (relative to current dir)
    pub working_directory: PathBuf,
    /// Whether to create backups before modifications
    pub auto_backup: bool,
    /// Whether to run in dry-run mode (preview only)
    pub dry_run_mode: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        let working_directory = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            enabled: false,
            allowed_extensions: vec![
                "txt".to_string(),
                "md".to_string(),
                "rs".to_string(),
                "toml".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "js".to_string(),
                "ts".to_string(),
                "py".to_string(),
                "html".to_string(),
                "css".to_string(),
                "xml".to_string(),
                "csv".to_string(),
                "log".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10MB
            working_directory,
            auto_backup: true,
            dry_run_mode: false,
        }
    }
}

/// Agent state and execution context
#[derive(Debug)]
pub struct Agent {
    config: AgentConfig,
    executor: AgentExecutor,
    completion_detector: CompletionDetector,
    safety_manager: SafetyManager,
    tool_history: Vec<ToolCall>,
}

impl Agent {
    /// Create a new agent with the given configuration
    pub fn new(mut config: AgentConfig) -> Result<Self> {
        config.working_directory = normalize_working_directory(&config.working_directory)?;

        let safety_manager = SafetyManager::new(&config)?;
        let executor = AgentExecutor::new(config.clone(), safety_manager.clone())?;
        let completion_detector = CompletionDetector::new();

        Ok(Self {
            config,
            executor,
            completion_detector,
            safety_manager,
            tool_history: Vec::new(),
        })
    }

    /// Check if agent mode is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Enable or disable agent mode
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    /// Get current configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Update configuration
    pub fn update_config(&mut self, mut config: AgentConfig) -> Result<()> {
        config.working_directory = normalize_working_directory(&config.working_directory)?;
        self.safety_manager = SafetyManager::new(&config)?;
        self.executor = AgentExecutor::new(config.clone(), self.safety_manager.clone())?;
        self.config = config;
        Ok(())
    }

    /// Process a message and detect tool calls
    pub fn detect_tool_calls(&self, message: &str) -> Result<Vec<ToolCall>> {
        if !self.is_enabled() {
            return Ok(Vec::new());
        }

        // Try to parse structured tool calls from the message
        self.parse_tool_calls(message)
    }

    /// Execute a tool call
    pub async fn execute_tool(&mut self, tool_call: ToolCall) -> Result<ToolResult> {
        if !self.is_enabled() {
            return Err(anyhow!("Agent mode is not enabled"));
        }

        // Add to history
        self.tool_history.push(tool_call.clone());

        // Execute the tool and record activity
        match self.executor.execute(tool_call).await {
            Ok(result) => {
                self.completion_detector.record_tool_execution();
                Ok(result)
            }
            Err(e) => {
                self.completion_detector.record_tool_execution();
                Err(e)
            }
        }
    }

    /// Check if the current task appears to be complete
    pub fn is_task_complete(&self, recent_messages: &[String]) -> bool {
        if !self.is_enabled() {
            return false;
        }

        self.completion_status(recent_messages).is_complete()
    }

    /// Get the current completion status classification
    pub fn completion_status(&self, recent_messages: &[String]) -> CompletionStatus {
        self.completion_detector
            .completion_status(recent_messages, &self.tool_history)
    }

    /// Get the confidence score associated with the completion status
    pub fn completion_confidence(&self, recent_messages: &[String]) -> f64 {
        self.completion_detector
            .completion_confidence(recent_messages, &self.tool_history)
    }

    /// Describe which completion patterns currently match
    pub fn completion_pattern_matches(&self, recent_messages: &[String]) -> Vec<String> {
        self.completion_detector
            .matching_patterns(recent_messages, &self.tool_history)
    }

    /// Get tool execution history
    pub fn tool_history(&self) -> &[ToolCall] {
        &self.tool_history
    }

    /// Clear tool history
    pub fn clear_history(&mut self) {
        self.tool_history.clear();
    }

    /// Get available tools
    pub fn available_tools(&self) -> Vec<String> {
        self.executor.available_tools()
    }

    /// Get structured tool definitions for LLM function calling
    pub fn tool_definitions(&self) -> Vec<ToolDefinition> {
        self.executor
            .tool_infos()
            .into_iter()
            .map(|info| ToolDefinition::new(info.name, info.description, info.parameters))
            .collect()
    }

    /// Get detailed descriptions for available tools
    pub fn tool_catalog(&self) -> Vec<String> {
        self.available_tools()
            .into_iter()
            .filter_map(|name| {
                self.executor
                    .get_tool_info(&name)
                    .map(|info| info.format_description())
            })
            .collect()
    }

    /// Add an allowed path to the safety manager at runtime
    pub fn add_allowed_path(&mut self, path: PathBuf) {
        self.safety_manager.add_allowed_path(path);
    }

    /// Add a forbidden path to the safety manager at runtime
    pub fn add_forbidden_path(&mut self, path: PathBuf) {
        self.safety_manager.add_forbidden_path(path);
    }

    /// Get the configured allowed paths
    pub fn allowed_paths(&self) -> Vec<PathBuf> {
        self.safety_manager.allowed_paths().to_vec()
    }

    /// Get the configured forbidden paths
    pub fn forbidden_paths(&self) -> Vec<PathBuf> {
        self.safety_manager.forbidden_paths().to_vec()
    }

    /// Check if a path would be permitted by the safety manager
    pub fn is_path_allowed<P: AsRef<Path>>(&self, path: P) -> bool {
        self.safety_manager.would_allow_path(path.as_ref())
    }

    /// Get agent status summary
    pub fn status(&self) -> AgentStatus {
        AgentStatus {
            enabled: self.config.enabled,
            tools_executed: self.tool_history.len(),
            working_directory: self.config.working_directory.clone(),
            dry_run_mode: self.config.dry_run_mode,
            available_tools: self.available_tools(),
        }
    }

    /// Parse tool calls from a message
    fn parse_tool_calls(&self, message: &str) -> Result<Vec<ToolCall>> {
        let mut tool_calls = Vec::new();

        // Look for JSON-like tool call patterns
        if let Some(tool_call) = self.try_parse_json_tool_call(message)? {
            tool_calls.push(tool_call);
        }

        // Look for natural language tool requests
        tool_calls.extend(self.parse_natural_language_tools(message)?);

        Ok(tool_calls)
    }

    /// Try to parse a JSON-formatted tool call
    fn try_parse_json_tool_call(&self, message: &str) -> Result<Option<ToolCall>> {
        // Look for JSON blocks in the message
        for line in message.lines() {
            let line = line.trim();
            if line.starts_with('{') && line.ends_with('}') {
                if let Ok(tool_call) = serde_json::from_str::<ToolCall>(line) {
                    return Ok(Some(tool_call));
                }
            }
        }

        Ok(None)
    }

    /// Parse natural language tool requests
    fn parse_natural_language_tools(&self, message: &str) -> Result<Vec<ToolCall>> {
        let mut tool_calls = Vec::new();
        let message_lower = message.to_lowercase();

        // Simple pattern matching for common requests
        if message_lower.contains("read")
            && (message_lower.contains("file") || message_lower.contains("content"))
        {
            if let Some(path) = self.extract_file_path(&message_lower) {
                tool_calls.push(ToolCall {
                    tool: "read_file".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("path".to_string(), serde_json::Value::String(path));
                        params
                    },
                    thought: Some("Reading file content as requested".to_string()),
                    reasoning: Some("User requested to read a file".to_string()),
                });
            }
        }

        if message_lower.contains("search") || message_lower.contains("find") {
            if let Some(pattern) = self.extract_search_pattern(&message_lower) {
                tool_calls.push(ToolCall {
                    tool: "search_files".to_string(),
                    parameters: {
                        let mut params = HashMap::new();
                        params.insert("pattern".to_string(), serde_json::Value::String(pattern));
                        params.insert(
                            "directory".to_string(),
                            serde_json::Value::String(".".to_string()),
                        );
                        params
                    },
                    thought: Some("Searching for files as requested".to_string()),
                    reasoning: Some("User requested to search for files".to_string()),
                });
            }
        }

        if message_lower.contains("list")
            && (message_lower.contains("files") || message_lower.contains("directory"))
        {
            let directory = self
                .extract_directory_path(&message_lower)
                .unwrap_or_else(|| ".".to_string());
            tool_calls.push(ToolCall {
                tool: "list_directory".to_string(),
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("path".to_string(), serde_json::Value::String(directory));
                    params
                },
                thought: Some("Listing directory contents as requested".to_string()),
                reasoning: Some("User requested to list files or directory contents".to_string()),
            });
        }

        Ok(tool_calls)
    }

    /// Extract file path from message
    fn extract_file_path(&self, message: &str) -> Option<String> {
        // Simple extraction - look for common file patterns
        let words: Vec<&str> = message.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            if word.contains('.') && (word.contains('/') || !word.contains(' ')) {
                return Some(word.to_string());
            }
            // Look for quoted paths
            if word.starts_with('"') || word.starts_with('\'') {
                if let Some(end_idx) = words
                    .iter()
                    .skip(i)
                    .position(|w| w.ends_with('"') || w.ends_with('\''))
                {
                    let path_parts: Vec<&str> =
                        words.iter().skip(i).take(end_idx + 1).cloned().collect();
                    let path = path_parts.join(" ");
                    return Some(path.trim_matches(|c| c == '"' || c == '\'').to_string());
                }
            }
        }
        None
    }

    /// Extract search pattern from message
    fn extract_search_pattern(&self, message: &str) -> Option<String> {
        // Look for quoted search terms
        if let Some(start) = message.find('"') {
            if let Some(end) = message[start + 1..].find('"') {
                return Some(message[start + 1..start + 1 + end].to_string());
            }
        }
        if let Some(start) = message.find('\'') {
            if let Some(end) = message[start + 1..].find('\'') {
                return Some(message[start + 1..start + 1 + end].to_string());
            }
        }

        // Look for "for X" patterns
        if let Some(for_pos) = message.find(" for ") {
            let after_for = &message[for_pos + 5..];
            if let Some(end) = after_for.find(' ') {
                return Some(after_for[..end].to_string());
            } else {
                return Some(after_for.to_string());
            }
        }

        None
    }

    /// Extract directory path from message
    fn extract_directory_path(&self, message: &str) -> Option<String> {
        self.extract_file_path(message)
    }
}

/// Agent status information
#[derive(Debug, Serialize)]
pub struct AgentStatus {
    pub enabled: bool,
    pub tools_executed: usize,
    pub working_directory: PathBuf,
    pub dry_run_mode: bool,
    pub available_tools: Vec<String>,
}

fn normalize_working_directory(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_working_directory_converts_relative_path() {
        let normalized = normalize_working_directory(Path::new(".")).unwrap();
        assert!(normalized.is_absolute());
    }

    #[test]
    fn normalize_working_directory_preserves_absolute_path() {
        let absolute = std::env::temp_dir();
        let normalized = normalize_working_directory(absolute.as_path()).unwrap();
        assert_eq!(normalized, absolute);
    }
}
