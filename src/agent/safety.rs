//! Safety and sandboxing module for agent operations
//! 
//! Provides security checks and restrictions to ensure safe file operations.

use super::{AgentConfig, ToolCall};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Safety manager for agent operations
#[derive(Debug, Clone)]
pub struct SafetyManager {
    config: AgentConfig,
    allowed_paths: Vec<PathBuf>,
    forbidden_paths: Vec<PathBuf>,
}

impl SafetyManager {
    /// Create a new safety manager with the given configuration
    pub fn new(config: &AgentConfig) -> Result<Self> {
        let mut manager = Self {
            config: config.clone(),
            allowed_paths: Vec::new(),
            forbidden_paths: Vec::new(),
        };

        // Set up default allowed and forbidden paths
        manager.setup_default_restrictions()?;

        Ok(manager)
    }

    /// Set up default path restrictions
    fn setup_default_restrictions(&mut self) -> Result<()> {
        // Allow operations in the working directory and subdirectories
        self.allowed_paths.push(self.config.working_directory.clone());

        // Forbidden system paths
        let forbidden = [
            "/etc",
            "/usr",
            "/bin",
            "/sbin",
            "/boot",
            "/dev",
            "/proc",
            "/sys",
            "/var/log",
            "/var/lib",
            "/root",
            "/home/*/.ssh",
            "/home/*/.gnupg",
            "C:\\Windows",
            "C:\\Program Files",
            "C:\\Program Files (x86)",
            "C:\\System32",
        ];

        for path in &forbidden {
            self.forbidden_paths.push(PathBuf::from(path));
        }

        Ok(())
    }

    /// Check if a tool call is safe to execute
    pub fn check_tool_call(&self, tool_call: &ToolCall) -> Result<()> {
        // Check file path restrictions for file operations
        if self.is_file_operation(&tool_call.tool) {
            self.check_file_path_safety(tool_call)?;
        }

        // Check file size restrictions
        if tool_call.tool == "write_file" || tool_call.tool == "update_file" {
            self.check_file_size_limits(tool_call)?;
        }

        // Check file extension restrictions
        if self.is_file_operation(&tool_call.tool) {
            self.check_file_extension(tool_call)?;
        }

        // Check for potentially dangerous content
        if tool_call.tool == "write_file" || tool_call.tool == "update_file" {
            self.check_content_safety(tool_call)?;
        }

        Ok(())
    }

    /// Check if a tool operates on files
    fn is_file_operation(&self, tool_name: &str) -> bool {
        matches!(
            tool_name,
            "read_file" | "write_file" | "update_file" | "file_info"
        )
    }

    /// Check file path safety
    fn check_file_path_safety(&self, tool_call: &ToolCall) -> Result<()> {
        let path = tool_call
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing path parameter"))?;

        let path = Path::new(path);

        // Convert to absolute path for checking
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.working_directory.join(path)
        };

        // Normalize the path to resolve .. and . components
        let normalized_path = self.normalize_path(&abs_path)?;

        // Check if path is within allowed directories
        if !self.is_path_allowed(&normalized_path)? {
            return Err(anyhow!(
                "Path '{}' is outside allowed directories",
                normalized_path.display()
            ));
        }

        // Check if path is explicitly forbidden
        if self.is_path_forbidden(&normalized_path)? {
            return Err(anyhow!(
                "Path '{}' is in a forbidden directory",
                normalized_path.display()
            ));
        }

        // Check for path traversal attempts
        if path.to_string_lossy().contains("..") {
            return Err(anyhow!("Path traversal detected: {}", path.display()));
        }

        // Check for suspicious path patterns
        self.check_suspicious_paths(&normalized_path)?;

        Ok(())
    }

    /// Check if a path is within allowed directories
    fn is_path_allowed(&self, path: &Path) -> Result<bool> {
        for allowed in &self.allowed_paths {
            let allowed_abs = if allowed.is_absolute() {
                allowed.clone()
            } else {
                std::env::current_dir()?.join(allowed)
            };

            let normalized_allowed = self.normalize_path(&allowed_abs)?;

            if path.starts_with(&normalized_allowed) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check if a path is explicitly forbidden
    fn is_path_forbidden(&self, path: &Path) -> Result<bool> {
        for forbidden in &self.forbidden_paths {
            // Handle wildcard patterns
            if forbidden.to_string_lossy().contains('*') {
                if self.matches_wildcard_pattern(path, forbidden)? {
                    return Ok(true);
                }
            } else if path.starts_with(forbidden) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Check for suspicious path patterns
    fn check_suspicious_paths(&self, path: &Path) -> Result<()> {
        let path_str = path.to_string_lossy().to_lowercase();

        // Check for common sensitive files
        let sensitive_patterns = [
            "passwd", "shadow", "hosts", "sudoers", "ssh_config", "authorized_keys",
            "id_rsa", "id_dsa", "id_ecdsa", "id_ed25519", ".env", "config.json",
            "database.yml", "secrets.yml", "private.key", "certificate.pem",
        ];

        for pattern in &sensitive_patterns {
            if path_str.contains(pattern) {
                return Err(anyhow!(
                    "Access to potentially sensitive file '{}' is not allowed",
                    path.display()
                ));
            }
        }

        Ok(())
    }

    /// Check file size limits
    fn check_file_size_limits(&self, tool_call: &ToolCall) -> Result<()> {
        if let Some(content) = tool_call.parameters.get("content").and_then(|v| v.as_str()) {
            if content.len() > self.config.max_file_size {
                return Err(anyhow!(
                    "Content size ({} bytes) exceeds maximum allowed size ({} bytes)",
                    content.len(),
                    self.config.max_file_size
                ));
            }
        }

        Ok(())
    }

    /// Check file extension restrictions
    fn check_file_extension(&self, tool_call: &ToolCall) -> Result<()> {
        let path = tool_call
            .parameters
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing path parameter"))?;

        let path = Path::new(path);

        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            let ext_lower = extension.to_lowercase();
            
            if !self.config.allowed_extensions.contains(&ext_lower) {
                return Err(anyhow!(
                    "File extension '{}' is not allowed. Allowed extensions: {}",
                    extension,
                    self.config.allowed_extensions.join(", ")
                ));
            }
        }

        Ok(())
    }

    /// Check content for potentially dangerous patterns
    fn check_content_safety(&self, tool_call: &ToolCall) -> Result<()> {
        if let Some(content) = tool_call.parameters.get("content").and_then(|v| v.as_str()) {
            // Check for potentially dangerous content patterns
            let dangerous_patterns = [
                "rm -rf", "del /s", "format c:", "dd if=", ":(){ :|:& };:",
                "sudo rm", "chmod 777", "wget http", "curl http",
                "eval(", "exec(", "system(", "shell_exec(",
                "<script", "javascript:", "data:text/html",
            ];

            let content_lower = content.to_lowercase();
            for pattern in &dangerous_patterns {
                if content_lower.contains(pattern) {
                    return Err(anyhow!(
                        "Content contains potentially dangerous pattern: '{}'",
                        pattern
                    ));
                }
            }

            // Check for excessive binary content
            let binary_chars = content.chars().filter(|c| c.is_control() && *c != '\n' && *c != '\r' && *c != '\t').count();
            let binary_ratio = binary_chars as f64 / content.len() as f64;
            
            if binary_ratio > 0.1 {
                return Err(anyhow!(
                    "Content appears to contain binary data ({}% non-text characters)",
                    (binary_ratio * 100.0) as u32
                ));
            }
        }

        Ok(())
    }

    /// Normalize a path by resolving . and .. components
    fn normalize_path(&self, path: &Path) -> Result<PathBuf> {
        let mut components = Vec::new();
        
        for component in path.components() {
            match component {
                std::path::Component::Normal(name) => {
                    components.push(name);
                }
                std::path::Component::ParentDir => {
                    if components.is_empty() {
                        return Err(anyhow!("Path traversal outside root directory"));
                    }
                    components.pop();
                }
                std::path::Component::CurDir => {
                    // Skip current directory references
                }
                std::path::Component::RootDir => {
                    components.clear();
                    components.push(std::ffi::OsStr::new("/"));
                }
                std::path::Component::Prefix(prefix) => {
                    components.push(prefix.as_os_str());
                }
            }
        }

        let mut result = PathBuf::new();
        for component in components {
            result.push(component);
        }

        Ok(result)
    }

    /// Check if a path matches a wildcard pattern
    fn matches_wildcard_pattern(&self, path: &Path, pattern: &Path) -> Result<bool> {
        let path_str = path.to_string_lossy();
        let pattern_str = pattern.to_string_lossy();

        // Simple wildcard matching - convert * to regex .*
        let regex_pattern = pattern_str.replace('*', ".*");
        
        if let Ok(regex) = regex::Regex::new(&regex_pattern) {
            Ok(regex.is_match(&path_str))
        } else {
            Ok(false)
        }
    }

    /// Add an allowed path
    pub fn add_allowed_path(&mut self, path: PathBuf) {
        self.allowed_paths.push(path);
    }

    /// Add a forbidden path
    pub fn add_forbidden_path(&mut self, path: PathBuf) {
        self.forbidden_paths.push(path);
    }

    /// Get current allowed paths
    pub fn allowed_paths(&self) -> &[PathBuf] {
        &self.allowed_paths
    }

    /// Get current forbidden paths
    pub fn forbidden_paths(&self) -> &[PathBuf] {
        &self.forbidden_paths
    }

    /// Check if a specific path would be allowed
    pub fn would_allow_path(&self, path: &Path) -> bool {
        let abs_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.config.working_directory.join(path)
        };

        if let Ok(normalized_path) = self.normalize_path(&abs_path) {
            self.is_path_allowed(&normalized_path).unwrap_or(false)
                && !self.is_path_forbidden(&normalized_path).unwrap_or(true)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> AgentConfig {
        AgentConfig {
            enabled: true,
            allowed_extensions: vec!["txt".to_string(), "md".to_string()],
            max_file_size: 1024,
            working_directory: PathBuf::from("/tmp/test"),
            auto_backup: true,
            dry_run_mode: false,
        }
    }

    #[test]
    fn test_path_traversal_detection() {
        let config = create_test_config();
        let safety = SafetyManager::new(&config).unwrap();

        let mut params = HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String("../../../etc/passwd".to_string()));

        let tool_call = ToolCall {
            tool: "read_file".to_string(),
            parameters: params,
            thought: None,
            reasoning: None,
        };

        assert!(safety.check_tool_call(&tool_call).is_err());
    }

    #[test]
    fn test_file_extension_validation() {
        let config = create_test_config();
        let safety = SafetyManager::new(&config).unwrap();

        let mut params = HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String("test.exe".to_string()));

        let tool_call = ToolCall {
            tool: "read_file".to_string(),
            parameters: params,
            thought: None,
            reasoning: None,
        };

        assert!(safety.check_tool_call(&tool_call).is_err());
    }

    #[test]
    fn test_content_size_validation() {
        let config = create_test_config();
        let safety = SafetyManager::new(&config).unwrap();

        let large_content = "x".repeat(2048); // Exceeds max_file_size of 1024

        let mut params = HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String("test.txt".to_string()));
        params.insert("content".to_string(), serde_json::Value::String(large_content));

        let tool_call = ToolCall {
            tool: "write_file".to_string(),
            parameters: params,
            thought: None,
            reasoning: None,
        };

        assert!(safety.check_tool_call(&tool_call).is_err());
    }

    #[test]
    fn test_dangerous_content_detection() {
        let config = create_test_config();
        let safety = SafetyManager::new(&config).unwrap();

        let mut params = HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String("test.txt".to_string()));
        params.insert("content".to_string(), serde_json::Value::String("rm -rf /".to_string()));

        let tool_call = ToolCall {
            tool: "write_file".to_string(),
            parameters: params,
            thought: None,
            reasoning: None,
        };

        assert!(safety.check_tool_call(&tool_call).is_err());
    }
}
