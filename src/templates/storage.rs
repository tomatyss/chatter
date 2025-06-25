//! Template storage functionality
//! 
//! Handles file I/O operations for template persistence.

use super::Template;
use anyhow::{anyhow, Result};
use dirs::config_dir;
use std::fs;
use std::path::PathBuf;

/// Template storage manager
pub struct TemplateStorage {
    templates_dir: PathBuf,
}

impl TemplateStorage {
    /// Create a new template storage manager
    pub async fn new() -> Result<Self> {
        let templates_dir = get_templates_dir();
        
        // Create templates directory if it doesn't exist
        fs::create_dir_all(&templates_dir)?;
        
        Ok(Self { templates_dir })
    }

    /// Load all user templates from storage
    pub async fn load_all(&self) -> Result<Vec<Template>> {
        let mut templates = Vec::new();
        
        if !self.templates_dir.exists() {
            return Ok(templates);
        }

        let entries = fs::read_dir(&self.templates_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_template(&path).await {
                    Ok(template) => templates.push(template),
                    Err(e) => {
                        eprintln!("Warning: Failed to load template from {}: {}", path.display(), e);
                    }
                }
            }
        }
        
        Ok(templates)
    }

    /// Load a single template from file
    async fn load_template(&self, path: &PathBuf) -> Result<Template> {
        let content = fs::read_to_string(path)?;
        let template: Template = serde_json::from_str(&content)?;
        Ok(template)
    }

    /// Save a template to storage
    pub async fn save(&self, template: &Template) -> Result<()> {
        if template.builtin {
            return Err(anyhow!("Cannot save built-in templates to storage"));
        }

        let filename = format!("{}.json", sanitize_filename(&template.name));
        let path = self.templates_dir.join(filename);
        
        let content = serde_json::to_string_pretty(template)?;
        fs::write(&path, content)?;
        
        Ok(())
    }

    /// Delete a template from storage
    pub async fn delete(&self, name: &str) -> Result<()> {
        let filename = format!("{}.json", sanitize_filename(name));
        let path = self.templates_dir.join(filename);
        
        if !path.exists() {
            return Err(anyhow!("Template file not found: {}", path.display()));
        }
        
        fs::remove_file(&path)?;
        Ok(())
    }

    /// Check if a template exists in storage
    pub fn exists(&self, name: &str) -> bool {
        let filename = format!("{}.json", sanitize_filename(name));
        let path = self.templates_dir.join(filename);
        path.exists()
    }

    /// Get the path to a template file
    pub fn get_template_path(&self, name: &str) -> PathBuf {
        let filename = format!("{}.json", sanitize_filename(name));
        self.templates_dir.join(filename)
    }

    /// Get the templates directory path
    pub fn get_templates_dir(&self) -> &PathBuf {
        &self.templates_dir
    }
}

/// Get the templates directory path
fn get_templates_dir() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("chatter")
        .join("templates")
}

/// Sanitize a filename by replacing invalid characters
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal_name"), "normal_name");
        assert_eq!(sanitize_filename("name/with/slashes"), "name_with_slashes");
        assert_eq!(sanitize_filename("name:with:colons"), "name_with_colons");
        assert_eq!(sanitize_filename("name*with*stars"), "name_with_stars");
        assert_eq!(sanitize_filename("name\"with\"quotes"), "name_with_quotes");
    }
}
