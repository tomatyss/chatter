//! System instruction template management module
//! 
//! Provides functionality for creating, storing, and managing reusable system instruction templates.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod storage;
pub mod builtin;

pub use storage::TemplateStorage;
pub use builtin::get_builtin_templates;

/// A system instruction template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Template name (unique identifier)
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// The system instruction content
    pub content: String,
    /// Template category for organization
    pub category: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modified timestamp
    pub updated_at: DateTime<Utc>,
    /// Tags for searching and filtering
    pub tags: Vec<String>,
    /// Whether this is a built-in template
    pub builtin: bool,
}

impl Template {
    /// Create a new template
    pub fn new(
        name: String,
        description: String,
        content: String,
        category: String,
        tags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            name,
            description,
            content,
            category,
            created_at: now,
            updated_at: now,
            tags,
            builtin: false,
        }
    }

    /// Create a built-in template
    pub fn builtin(
        name: String,
        description: String,
        content: String,
        category: String,
        tags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            name,
            description,
            content,
            category,
            created_at: now,
            updated_at: now,
            tags,
            builtin: true,
        }
    }

    /// Update the template content
    pub fn update_content(&mut self, content: String) {
        self.content = content;
        self.updated_at = Utc::now();
    }

    /// Update the template description
    pub fn update_description(&mut self, description: String) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    /// Add a tag to the template
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag from the template
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Check if template matches search query
    pub fn matches_search(&self, query: &str) -> bool {
        let query = query.to_lowercase();
        self.name.to_lowercase().contains(&query)
            || self.description.to_lowercase().contains(&query)
            || self.category.to_lowercase().contains(&query)
            || self.tags.iter().any(|tag| tag.to_lowercase().contains(&query))
    }
}

/// Template manager for handling all template operations
pub struct TemplateManager {
    storage: TemplateStorage,
    templates: HashMap<String, Template>,
}

impl TemplateManager {
    /// Create a new template manager
    pub async fn new() -> Result<Self> {
        let storage = TemplateStorage::new().await?;
        let mut manager = Self {
            storage,
            templates: HashMap::new(),
        };
        
        // Load all templates
        manager.reload().await?;
        
        Ok(manager)
    }

    /// Reload all templates from storage
    pub async fn reload(&mut self) -> Result<()> {
        self.templates.clear();
        
        // Load built-in templates
        for template in get_builtin_templates() {
            self.templates.insert(template.name.clone(), template);
        }
        
        // Load user templates
        let user_templates = self.storage.load_all().await?;
        for template in user_templates {
            self.templates.insert(template.name.clone(), template);
        }
        
        Ok(())
    }

    /// Get all templates
    pub fn list_all(&self) -> Vec<&Template> {
        self.templates.values().collect()
    }

    /// Get templates by category
    pub fn list_by_category(&self, category: &str) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Search templates by query
    pub fn search(&self, query: &str) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| t.matches_search(query))
            .collect()
    }

    /// Get a template by name
    pub fn get(&self, name: &str) -> Option<&Template> {
        self.templates.get(name)
    }

    /// Create a new template
    pub async fn create(&mut self, template: Template) -> Result<()> {
        if self.templates.contains_key(&template.name) {
            return Err(anyhow!("Template '{}' already exists", template.name));
        }

        if template.builtin {
            return Err(anyhow!("Cannot create built-in templates"));
        }

        // Save to storage
        self.storage.save(&template).await?;
        
        // Add to memory
        self.templates.insert(template.name.clone(), template);
        
        Ok(())
    }

    /// Update an existing template
    pub async fn update(&mut self, name: &str, mut template: Template) -> Result<()> {
        let existing = self.templates.get(name)
            .ok_or_else(|| anyhow!("Template '{}' not found", name))?;

        if existing.builtin {
            return Err(anyhow!("Cannot modify built-in template '{}'", name));
        }

        // Preserve creation time
        template.created_at = existing.created_at;
        template.updated_at = Utc::now();

        // Save to storage
        self.storage.save(&template).await?;
        
        // Update in memory
        self.templates.insert(template.name.clone(), template);
        
        Ok(())
    }

    /// Delete a template
    pub async fn delete(&mut self, name: &str) -> Result<()> {
        let template = self.templates.get(name)
            .ok_or_else(|| anyhow!("Template '{}' not found", name))?;

        if template.builtin {
            return Err(anyhow!("Cannot delete built-in template '{}'", name));
        }

        // Remove from storage
        self.storage.delete(name).await?;
        
        // Remove from memory
        self.templates.remove(name);
        
        Ok(())
    }

    /// Get all unique categories
    pub fn get_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.templates
            .values()
            .map(|t| t.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }

    /// Get all unique tags
    pub fn get_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self.templates
            .values()
            .flat_map(|t| t.tags.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        tags.sort();
        tags
    }
}
