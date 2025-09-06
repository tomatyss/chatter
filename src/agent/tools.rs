//! Tool definitions and implementations for agent operations
//! 
//! Provides safe file operations, search capabilities, and other utilities
//! for autonomous task execution.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use regex::Regex;

/// A tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Name of the tool to execute
    pub tool: String,
    /// Parameters for the tool
    pub parameters: HashMap<String, serde_json::Value>,
    /// Optional thought process
    pub thought: Option<String>,
    /// Optional reasoning for the tool call
    pub reasoning: Option<String>,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the tool execution was successful
    pub success: bool,
    /// Result data or error message
    pub data: serde_json::Value,
    /// Optional message describing the result
    pub message: Option<String>,
    /// Files that were modified (for backup purposes)
    pub modified_files: Vec<PathBuf>,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(data: serde_json::Value, message: Option<String>) -> Self {
        Self {
            success: true,
            data,
            message,
            modified_files: Vec::new(),
        }
    }

    /// Create a successful result with modified files
    pub fn success_with_files(
        data: serde_json::Value,
        message: Option<String>,
        modified_files: Vec<PathBuf>,
    ) -> Self {
        Self {
            success: true,
            data,
            message,
            modified_files,
        }
    }

    /// Create an error result
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: serde_json::Value::String(message.clone()),
            message: Some(message),
            modified_files: Vec::new(),
        }
    }
}

/// Enum containing all available tools
#[derive(Debug)]
pub enum Tool {
    ReadFile(ReadFileTool),
    WriteFile(WriteFileTool),
    UpdateFile(UpdateFileTool),
    SearchFiles(SearchFilesTool),
    ListDirectory(ListDirectoryTool),
    FileInfo(FileInfoTool),
}

impl Tool {
    /// Get the name of this tool
    pub fn name(&self) -> &str {
        match self {
            Tool::ReadFile(tool) => tool.name(),
            Tool::WriteFile(tool) => tool.name(),
            Tool::UpdateFile(tool) => tool.name(),
            Tool::SearchFiles(tool) => tool.name(),
            Tool::ListDirectory(tool) => tool.name(),
            Tool::FileInfo(tool) => tool.name(),
        }
    }

    /// Get a description of what this tool does
    pub fn description(&self) -> &str {
        match self {
            Tool::ReadFile(tool) => tool.description(),
            Tool::WriteFile(tool) => tool.description(),
            Tool::UpdateFile(tool) => tool.description(),
            Tool::SearchFiles(tool) => tool.description(),
            Tool::ListDirectory(tool) => tool.description(),
            Tool::FileInfo(tool) => tool.description(),
        }
    }

    /// Get the parameter schema for this tool
    pub fn parameters(&self) -> serde_json::Value {
        match self {
            Tool::ReadFile(tool) => tool.parameters(),
            Tool::WriteFile(tool) => tool.parameters(),
            Tool::UpdateFile(tool) => tool.parameters(),
            Tool::SearchFiles(tool) => tool.parameters(),
            Tool::ListDirectory(tool) => tool.parameters(),
            Tool::FileInfo(tool) => tool.parameters(),
        }
    }

    /// Execute the tool with the given parameters
    pub async fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<ToolResult> {
        match self {
            Tool::ReadFile(tool) => tool.execute(parameters).await,
            Tool::WriteFile(tool) => tool.execute(parameters).await,
            Tool::UpdateFile(tool) => tool.execute(parameters).await,
            Tool::SearchFiles(tool) => tool.execute(parameters).await,
            Tool::ListDirectory(tool) => tool.execute(parameters).await,
            Tool::FileInfo(tool) => tool.execute(parameters).await,
        }
    }
}

/// Trait for implementing individual tool types
pub trait ToolImpl: Send + Sync {
    /// Get the name of this tool
    fn name(&self) -> &str;

    /// Get a description of what this tool does
    fn description(&self) -> &str;

    /// Get the parameter schema for this tool
    fn parameters(&self) -> serde_json::Value;

    /// Execute the tool with the given parameters
    async fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<ToolResult>;
}

/// Tool for reading file contents
#[derive(Debug)]
pub struct ReadFileTool;

impl ToolImpl for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a text file"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<ToolResult> {
        let path = parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'path' parameter"))?;

        let path = Path::new(path);
        
        if !path.exists() {
            return Ok(ToolResult::error(format!("File does not exist: {}", path.display())));
        }

        if !path.is_file() {
            return Ok(ToolResult::error(format!("Path is not a file: {}", path.display())));
        }

        match fs::read_to_string(path) {
            Ok(content) => {
                let result = serde_json::json!({
                    "path": path.display().to_string(),
                    "content": content,
                    "size": content.len()
                });
                Ok(ToolResult::success(
                    result,
                    Some(format!("Successfully read {} bytes from {}", content.len(), path.display())),
                ))
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to read file: {e}"))),
        }
    }
}

/// Tool for writing file contents
#[derive(Debug)]
pub struct WriteFileTool;

impl ToolImpl for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file (creates or overwrites)"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<ToolResult> {
        let path = parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'path' parameter"))?;

        let content = parameters
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'content' parameter"))?;

        let path = Path::new(path);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return Ok(ToolResult::error(format!("Failed to create directories: {e}")));
                }
            }
        }

        match fs::write(path, content) {
            Ok(()) => {
                let result = serde_json::json!({
                    "path": path.display().to_string(),
                    "size": content.len()
                });
                Ok(ToolResult::success_with_files(
                    result,
                    Some(format!("Successfully wrote {} bytes to {}", content.len(), path.display())),
                    vec![path.to_path_buf()],
                ))
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to write file: {e}"))),
        }
    }
}

/// Tool for updating file contents with targeted changes
#[derive(Debug)]
pub struct UpdateFileTool;

impl ToolImpl for UpdateFileTool {
    fn name(&self) -> &str {
        "update_file"
    }

    fn description(&self) -> &str {
        "Update a file by replacing specific content or appending to it"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to update"
                },
                "operation": {
                    "type": "string",
                    "enum": ["replace", "append", "prepend", "insert_at_line"],
                    "description": "Type of update operation"
                },
                "search": {
                    "type": "string",
                    "description": "Text to search for (required for replace operation)"
                },
                "replacement": {
                    "type": "string",
                    "description": "Replacement text (for replace operation) or content to add"
                },
                "line_number": {
                    "type": "integer",
                    "description": "Line number for insert_at_line operation (1-based)"
                }
            },
            "required": ["path", "operation"]
        })
    }

    async fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<ToolResult> {
        let path = parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'path' parameter"))?;

        let operation = parameters
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'operation' parameter"))?;

        let path = Path::new(path);

        if !path.exists() {
            return Ok(ToolResult::error(format!("File does not exist: {}", path.display())));
        }

        let original_content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => return Ok(ToolResult::error(format!("Failed to read file: {e}"))),
        };

        let new_content = match operation {
            "replace" => {
                let search = parameters
                    .get("search")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'search' parameter for replace operation"))?;

                let replacement = parameters
                    .get("replacement")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                original_content.replace(search, replacement)
            }
            "append" => {
                let content_to_add = parameters
                    .get("replacement")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'replacement' parameter for append operation"))?;

                format!("{original_content}\n{content_to_add}")
            }
            "prepend" => {
                let content_to_add = parameters
                    .get("replacement")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'replacement' parameter for prepend operation"))?;

                format!("{content_to_add}\n{original_content}")
            }
            "insert_at_line" => {
                let line_number = parameters
                    .get("line_number")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| anyhow!("Missing or invalid 'line_number' parameter"))?;

                let content_to_add = parameters
                    .get("replacement")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'replacement' parameter for insert_at_line operation"))?;

                let mut lines: Vec<&str> = original_content.lines().collect();
                let insert_index = (line_number as usize).saturating_sub(1);
                
                if insert_index <= lines.len() {
                    lines.insert(insert_index, content_to_add);
                    lines.join("\n")
                } else {
                    return Ok(ToolResult::error(format!("Line number {line_number} is out of range")));
                }
            }
            _ => return Ok(ToolResult::error(format!("Unknown operation: {operation}"))),
        };

        match fs::write(path, &new_content) {
            Ok(()) => {
                let result = serde_json::json!({
                    "path": path.display().to_string(),
                    "operation": operation,
                    "original_size": original_content.len(),
                    "new_size": new_content.len()
                });
                Ok(ToolResult::success_with_files(
                    result,
                    Some(format!("Successfully updated {} using {} operation", path.display(), operation)),
                    vec![path.to_path_buf()],
                ))
            }
            Err(e) => Ok(ToolResult::error(format!("Failed to update file: {e}"))),
        }
    }
}

/// Tool for searching files
#[derive(Debug)]
pub struct SearchFilesTool;

impl ToolImpl for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }

    fn description(&self) -> &str {
        "Search for text patterns across files in a directory"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Text pattern or regex to search for"
                },
                "directory": {
                    "type": "string",
                    "description": "Directory to search in (default: current directory)"
                },
                "file_pattern": {
                    "type": "string",
                    "description": "File name pattern to filter (e.g., '*.rs', '*.txt')"
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether the search should be case sensitive (default: false)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 100)"
                }
            },
            "required": ["pattern"]
        })
    }

    async fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<ToolResult> {
        let pattern = parameters
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'pattern' parameter"))?;

        let directory = parameters
            .get("directory")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let file_pattern = parameters
            .get("file_pattern")
            .and_then(|v| v.as_str());

        let case_sensitive = parameters
            .get("case_sensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let max_results = parameters
            .get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        let regex_pattern = if case_sensitive {
            match Regex::new(pattern) {
                Ok(r) => r,
                Err(_) => {
                    // If regex fails, treat as literal string
                    match Regex::new(&regex::escape(pattern)) {
                        Ok(r) => r,
                        Err(e) => return Ok(ToolResult::error(format!("Invalid pattern: {e}"))),
                    }
                }
            }
        } else {
            match Regex::new(&format!("(?i){pattern}")) {
                Ok(r) => r,
                Err(_) => {
                    // If regex fails, treat as literal string
                    match Regex::new(&format!("(?i){}", regex::escape(pattern))) {
                        Ok(r) => r,
                        Err(e) => return Ok(ToolResult::error(format!("Invalid pattern: {e}"))),
                    }
                }
            }
        };

        let mut results = Vec::new();
        let mut files_searched = 0;

        for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();
            
            // Apply file pattern filter if specified
            if let Some(file_pat) = file_pattern {
                if !path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| glob_match(file_pat, n))
                    .unwrap_or(false)
                {
                    continue;
                }
            }

            // Only search text files
            if !is_text_file(path) {
                continue;
            }

            files_searched += 1;

            if let Ok(content) = fs::read_to_string(path) {
                for (line_num, line) in content.lines().enumerate() {
                    if regex_pattern.is_match(line) {
                        results.push(serde_json::json!({
                            "file": path.display().to_string(),
                            "line": line_num + 1,
                            "content": line,
                            "matches": regex_pattern.find_iter(line)
                                .map(|m| serde_json::json!({
                                    "start": m.start(),
                                    "end": m.end(),
                                    "text": m.as_str()
                                }))
                                .collect::<Vec<_>>()
                        }));

                        if results.len() >= max_results {
                            break;
                        }
                    }
                }
            }

            if results.len() >= max_results {
                break;
            }
        }

        let result = serde_json::json!({
            "pattern": pattern,
            "directory": directory,
            "files_searched": files_searched,
            "matches_found": results.len(),
            "results": results
        });

        Ok(ToolResult::success(
            result,
            Some(format!("Found {} matches in {} files", results.len(), files_searched)),
        ))
    }
}

/// Tool for listing directory contents
#[derive(Debug)]
pub struct ListDirectoryTool;

impl ToolImpl for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn description(&self) -> &str {
        "List files and directories in a given path"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory path to list (default: current directory)"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to list recursively (default: false)"
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Whether to show hidden files (default: false)"
                }
            }
        })
    }

    async fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<ToolResult> {
        let path = parameters
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let recursive = parameters
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let show_hidden = parameters
            .get("show_hidden")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let path = Path::new(path);

        if !path.exists() {
            return Ok(ToolResult::error(format!("Path does not exist: {}", path.display())));
        }

        if !path.is_dir() {
            return Ok(ToolResult::error(format!("Path is not a directory: {}", path.display())));
        }

        let mut entries = Vec::new();

        if recursive {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                let file_name = entry_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if !show_hidden && file_name.starts_with('.') && file_name != "." {
                    continue;
                }

                let metadata = entry.metadata().ok();
                entries.push(serde_json::json!({
                    "path": entry_path.display().to_string(),
                    "name": file_name,
                    "type": if entry.file_type().is_dir() { "directory" } else { "file" },
                    "size": metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                    "modified": metadata.as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                }));
            }
        } else {
            match fs::read_dir(path) {
                Ok(dir_entries) => {
                    for entry in dir_entries.filter_map(|e| e.ok()) {
                        let entry_path = entry.path();
                        let file_name = entry_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");

                        if !show_hidden && file_name.starts_with('.') {
                            continue;
                        }

                        let metadata = entry.metadata().ok();
                        entries.push(serde_json::json!({
                            "path": entry_path.display().to_string(),
                            "name": file_name,
                            "type": if entry_path.is_dir() { "directory" } else { "file" },
                            "size": metadata.as_ref().map(|m| m.len()).unwrap_or(0),
                            "modified": metadata.as_ref()
                                .and_then(|m| m.modified().ok())
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                        }));
                    }
                }
                Err(e) => return Ok(ToolResult::error(format!("Failed to read directory: {e}"))),
            }
        }

        let result = serde_json::json!({
            "path": path.display().to_string(),
            "recursive": recursive,
            "entry_count": entries.len(),
            "entries": entries
        });

        Ok(ToolResult::success(
            result,
            Some(format!("Listed {} entries in {}", entries.len(), path.display())),
        ))
    }
}

/// Tool for getting file information
#[derive(Debug)]
pub struct FileInfoTool;

impl ToolImpl for FileInfoTool {
    fn name(&self) -> &str {
        "file_info"
    }

    fn description(&self) -> &str {
        "Get detailed information about a file or directory"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, parameters: HashMap<String, serde_json::Value>) -> Result<ToolResult> {
        let path = parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'path' parameter"))?;

        let path = Path::new(path);

        if !path.exists() {
            return Ok(ToolResult::error(format!("Path does not exist: {}", path.display())));
        }

        let metadata = match path.metadata() {
            Ok(m) => m,
            Err(e) => return Ok(ToolResult::error(format!("Failed to get metadata: {e}"))),
        };

        let file_type = if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        };

        let mut result = serde_json::json!({
            "path": path.display().to_string(),
            "name": path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
            "type": file_type,
            "size": metadata.len(),
            "readonly": metadata.permissions().readonly(),
            "created": metadata.created().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            "modified": metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
            "accessed": metadata.accessed().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
        });

        // Add file-specific information
        if metadata.is_file() {
            if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
                result["extension"] = serde_json::Value::String(extension.to_string());
            }
            
            result["is_text"] = serde_json::Value::Bool(is_text_file(path));
            
            // For text files, add line count
            if is_text_file(path) {
                if let Ok(content) = fs::read_to_string(path) {
                    result["line_count"] = serde_json::Value::Number(
                        serde_json::Number::from(content.lines().count())
                    );
                }
            }
        }

        Ok(ToolResult::success(
            result,
            Some(format!("Retrieved information for {}", path.display())),
        ))
    }
}

/// Check if a file is likely a text file based on extension
fn is_text_file(path: &Path) -> bool {
    let text_extensions = [
        "txt", "md", "rs", "toml", "json", "yaml", "yml", "js", "ts", "py", 
        "html", "css", "xml", "csv", "log", "cfg", "conf", "ini", "sh", 
        "bash", "zsh", "fish", "ps1", "bat", "cmd", "c", "cpp", "h", "hpp",
        "java", "kt", "swift", "go", "rb", "php", "pl", "r", "sql", "dockerfile"
    ];

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| text_extensions.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

/// Simple glob pattern matching
fn glob_match(pattern: &str, text: &str) -> bool {
    // Convert glob pattern to regex
    let regex_pattern = pattern
        .replace(".", r"\.")
        .replace("*", ".*")
        .replace("?", ".");
    
    if let Ok(regex) = Regex::new(&format!("^{regex_pattern}$")) {
        regex.is_match(text)
    } else {
        false
    }
}
