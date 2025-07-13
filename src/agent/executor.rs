//! Tool execution engine for the agent
//! 
//! Manages tool registration, execution, and safety checks.

use super::{AgentConfig, SafetyManager, ToolCall, ToolResult};
use super::tools::{
    Tool, ReadFileTool, WriteFileTool, UpdateFileTool, SearchFilesTool, 
    ListDirectoryTool, FileInfoTool
};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Tool execution engine
#[derive(Debug)]
pub struct AgentExecutor {
    config: AgentConfig,
    safety_manager: SafetyManager,
    tools: HashMap<String, Tool>,
}

impl AgentExecutor {
    /// Create a new executor with the given configuration
    pub fn new(config: AgentConfig, safety_manager: SafetyManager) -> Result<Self> {
        let mut executor = Self {
            config,
            safety_manager,
            tools: HashMap::new(),
        };

        // Register built-in tools
        executor.register_builtin_tools()?;

        Ok(executor)
    }

    /// Register all built-in tools
    fn register_builtin_tools(&mut self) -> Result<()> {
        self.register_tool(Tool::ReadFile(ReadFileTool))?;
        self.register_tool(Tool::WriteFile(WriteFileTool))?;
        self.register_tool(Tool::UpdateFile(UpdateFileTool))?;
        self.register_tool(Tool::SearchFiles(SearchFilesTool))?;
        self.register_tool(Tool::ListDirectory(ListDirectoryTool))?;
        self.register_tool(Tool::FileInfo(FileInfoTool))?;

        Ok(())
    }

    /// Register a new tool
    pub fn register_tool(&mut self, tool: Tool) -> Result<()> {
        let name = tool.name().to_string();
        if self.tools.contains_key(&name) {
            return Err(anyhow!("Tool '{}' is already registered", name));
        }
        self.tools.insert(name, tool);
        Ok(())
    }

    /// Get a list of available tool names
    pub fn available_tools(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get tool information
    pub fn get_tool_info(&self, name: &str) -> Option<ToolInfo> {
        self.tools.get(name).map(|tool| ToolInfo {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            parameters: tool.parameters(),
        })
    }

    /// Get information for all tools
    pub fn get_all_tool_info(&self) -> Vec<ToolInfo> {
        self.tools
            .values()
            .map(|tool| ToolInfo {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters(),
            })
            .collect()
    }

    /// Execute a tool call
    pub async fn execute(&self, tool_call: ToolCall) -> Result<ToolResult> {
        // Check if tool exists
        let tool = self.tools.get(&tool_call.tool)
            .ok_or_else(|| anyhow!("Unknown tool: {}", tool_call.tool))?;

        // Perform safety checks
        if let Err(e) = self.safety_manager.check_tool_call(&tool_call) {
            return Ok(ToolResult::error(format!("Safety check failed: {}", e)));
        }

        // Execute in dry-run mode if configured
        if self.config.dry_run_mode {
            return self.execute_dry_run(tool, &tool_call).await;
        }

        // Create backup if this is a file modification operation
        let backup_info = if self.is_file_modification_tool(&tool_call.tool) {
            self.create_backup_if_needed(&tool_call).await?
        } else {
            None
        };

        // Execute the tool
        let mut result = match tool.execute(tool_call.parameters.clone()).await {
            Ok(result) => result,
            Err(e) => {
                // If execution failed and we created a backup, we might want to clean it up
                if let Some(_backup) = backup_info {
                    // For now, we'll keep the backup even on failure
                }
                return Ok(ToolResult::error(format!("Tool execution failed: {}", e)));
            }
        };

        // Add backup information to successful results
        if let Some(backup) = backup_info {
            if result.success {
                if let serde_json::Value::Object(ref mut obj) = result.data {
                    obj.insert("backup_created".to_string(), serde_json::Value::String(backup));
                }
            }
        }

        Ok(result)
    }

    /// Execute a tool in dry-run mode (preview only)
    async fn execute_dry_run(&self, tool: &Tool, tool_call: &ToolCall) -> Result<ToolResult> {
        let preview_data = serde_json::json!({
            "tool": tool_call.tool,
            "parameters": tool_call.parameters,
            "description": tool.description(),
            "dry_run": true,
            "note": "This is a preview - no actual changes were made"
        });

        Ok(ToolResult::success(
            preview_data,
            Some(format!("DRY RUN: Would execute {} with given parameters", tool_call.tool)),
        ))
    }

    /// Check if a tool modifies files
    fn is_file_modification_tool(&self, tool_name: &str) -> bool {
        matches!(tool_name, "write_file" | "update_file")
    }

    /// Create backup for file modification operations
    async fn create_backup_if_needed(&self, tool_call: &ToolCall) -> Result<Option<String>> {
        if !self.config.auto_backup {
            return Ok(None);
        }

        // Extract file path from parameters
        let path = tool_call.parameters.get("path")
            .and_then(|v| v.as_str());

        if let Some(file_path) = path {
            let path = std::path::Path::new(file_path);
            
            // Only create backup if file exists
            if path.exists() && path.is_file() {
                let backup_path = self.generate_backup_path(path)?;
                
                if let Err(e) = std::fs::copy(path, &backup_path) {
                    return Err(anyhow!("Failed to create backup: {}", e));
                }
                
                return Ok(Some(backup_path.display().to_string()));
            }
        }

        Ok(None)
    }

    /// Generate a unique backup file path
    fn generate_backup_path(&self, original_path: &std::path::Path) -> Result<std::path::PathBuf> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let file_name = original_path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("Invalid file name"))?;
        
        let backup_name = format!("{}.backup_{}", file_name, timestamp);
        
        let backup_path = if let Some(parent) = original_path.parent() {
            parent.join(backup_name)
        } else {
            std::path::PathBuf::from(backup_name)
        };

        Ok(backup_path)
    }

    /// Validate tool call parameters against tool schema
    pub fn validate_tool_call(&self, tool_call: &ToolCall) -> Result<()> {
        let tool = self.tools.get(&tool_call.tool)
            .ok_or_else(|| anyhow!("Unknown tool: {}", tool_call.tool))?;

        // Basic validation - check required parameters
        let schema = tool.parameters();
        if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
            if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
                for req_param in required {
                    if let Some(param_name) = req_param.as_str() {
                        if !tool_call.parameters.contains_key(param_name) {
                            return Err(anyhow!("Missing required parameter: {}", param_name));
                        }
                    }
                }
            }

            // Type validation for known parameters
            for (param_name, param_value) in &tool_call.parameters {
                if let Some(param_schema) = properties.get(param_name) {
                    self.validate_parameter_type(param_name, param_value, param_schema)?;
                }
            }
        }

        Ok(())
    }

    /// Validate parameter type against schema
    fn validate_parameter_type(
        &self,
        param_name: &str,
        value: &serde_json::Value,
        schema: &serde_json::Value,
    ) -> Result<()> {
        if let Some(expected_type) = schema.get("type").and_then(|t| t.as_str()) {
            let actual_type = match value {
                serde_json::Value::String(_) => "string",
                serde_json::Value::Number(_) => "number",
                serde_json::Value::Bool(_) => "boolean",
                serde_json::Value::Array(_) => "array",
                serde_json::Value::Object(_) => "object",
                serde_json::Value::Null => "null",
            };

            if expected_type != actual_type && !(expected_type == "integer" && actual_type == "number") {
                return Err(anyhow!(
                    "Parameter '{}' has type '{}' but expected '{}'",
                    param_name,
                    actual_type,
                    expected_type
                ));
            }
        }

        Ok(())
    }
}

/// Information about a tool
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ToolInfo {
    /// Get a human-readable description of the tool
    pub fn format_description(&self) -> String {
        let mut desc = format!("**{}**: {}", self.name, self.description);
        
        if let Some(properties) = self.parameters.get("properties").and_then(|p| p.as_object()) {
            desc.push_str("\n\nParameters:");
            
            let required_params = self.parameters.get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            
            for (param_name, param_info) in properties {
                let is_required = required_params.contains(&param_name.as_str());
                let param_type = param_info.get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown");
                let param_desc = param_info.get("description")
                    .and_then(|d| d.as_str())
                    .unwrap_or("No description");
                
                desc.push_str(&format!(
                    "\n  - {} ({}){}: {}",
                    param_name,
                    param_type,
                    if is_required { " *required*" } else { "" },
                    param_desc
                ));
            }
        }
        
        desc
    }
}
