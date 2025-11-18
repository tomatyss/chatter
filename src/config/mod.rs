//! Configuration management module
//!
//! Handles API key storage, user preferences, and configuration file management.

use anyhow::{anyhow, Result};
use dialoguer::Password;
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub mod settings;

/// Supported model providers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelProvider {
    Gemini,
    Ollama,
}

impl Default for ModelProvider {
    fn default() -> Self {
        Self::Gemini
    }
}

impl ModelProvider {
    /// Whether this provider requires an API key for authentication
    pub fn requires_api_key(&self) -> bool {
        matches!(self, Self::Gemini)
    }
}

/// Configuration specific to the Ollama provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Base URL for the Ollama server
    pub endpoint: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:11434".to_string(),
        }
    }
}

fn default_provider() -> ModelProvider {
    ModelProvider::default()
}

fn default_ollama_config() -> OllamaConfig {
    OllamaConfig::default()
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Gemini API key
    pub api_key: String,
    /// Default model to use
    pub default_model: String,
    /// Default system instruction
    pub default_system_instruction: Option<String>,
    /// Auto-save sessions
    pub auto_save: bool,
    /// Sessions directory
    pub sessions_dir: PathBuf,
    /// Preferred model provider
    #[serde(default = "default_provider")]
    pub provider: ModelProvider,
    /// Provider-specific configuration for Ollama
    #[serde(default = "default_ollama_config")]
    pub ollama: OllamaConfig,
}

impl Default for Config {
    fn default() -> Self {
        let config_dir = get_config_dir();
        Self {
            api_key: String::new(),
            default_model: "gemini-2.5-flash".to_string(),
            default_system_instruction: None,
            auto_save: false,
            sessions_dir: config_dir.join("sessions"),
            provider: ModelProvider::default(),
            ollama: OllamaConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from file or environment
    pub async fn load() -> Result<Self> {
        // Defer API key validation to the point where we actually need it so that
        // users can run Ollama-only workflows without configuring Gemini first.
        Self::load_with_api_key_required(false).await
    }

    /// Load configuration, optionally requiring an API key
    pub async fn load_with_api_key_required(require_api_key: bool) -> Result<Self> {
        // First try to load from config file
        if let Ok(config) = Self::load_from_file().await {
            if !require_api_key || !config.provider.requires_api_key() || !config.api_key.is_empty()
            {
                return Ok(config);
            }
        }

        // If no config file, create default and try to get API key from environment
        let mut config = Self::default();

        // Try to get API key from environment variable
        if config.provider.requires_api_key() {
            if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
                config.api_key = api_key;
            } else if require_api_key && config.api_key.is_empty() {
                return Err(anyhow!(
                    "No API key found. Please set GEMINI_API_KEY environment variable or run 'chatter config set-api-key'"
                ));
            }
        }

        Ok(config)
    }

    /// Load configuration from file
    async fn load_from_file() -> Result<Self> {
        let config_path = get_config_file_path();
        if !config_path.exists() {
            return Err(anyhow!("Config file not found"));
        }

        let content = fs::read_to_string(&config_path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub async fn save(&self) -> Result<()> {
        let config_dir = get_config_dir();
        fs::create_dir_all(&config_dir)?;

        let config_path = get_config_file_path();
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, content)?;

        // Also create sessions directory
        fs::create_dir_all(&self.sessions_dir)?;

        Ok(())
    }

    /// Set API key interactively
    pub async fn set_api_key_interactive(&mut self) -> Result<()> {
        println!("🔑 Setting up Gemini API Key");
        println!("You can get your API key from: https://aistudio.google.com/app/apikey");
        println!();

        let api_key: String = Password::new()
            .with_prompt("Enter your Gemini API key")
            .interact()?;

        if api_key.trim().is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        self.api_key = api_key.trim().to_string();
        self.save().await?;

        Ok(())
    }

    /// Display current configuration
    pub fn display(&self) {
        println!("📋 Current Configuration:");
        println!(
            "  Provider: {}",
            match self.provider {
                ModelProvider::Gemini => "Gemini",
                ModelProvider::Ollama => "Ollama",
            }
        );
        println!(
            "  API Key: {}",
            if self.api_key.is_empty() {
                "Not set"
            } else {
                "Set (hidden)"
            }
        );
        println!("  Default Model: {}", self.default_model);
        println!("  Auto-save: {}", self.auto_save);
        println!("  Sessions Directory: {}", self.sessions_dir.display());
        if let Some(ref system) = self.default_system_instruction {
            println!("  Default System Instruction: {system}");
        }
        if matches!(self.provider, ModelProvider::Ollama) {
            println!("  Ollama Endpoint: {}", self.ollama.endpoint);
        }
    }

    /// Reset configuration to defaults
    pub async fn reset(&mut self) -> Result<()> {
        *self = Self::default();

        // Remove config file if it exists
        let config_path = get_config_file_path();
        if config_path.exists() {
            fs::remove_file(&config_path)?;
        }

        Ok(())
    }
}

/// Get the configuration directory path
fn get_config_dir() -> PathBuf {
    config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("chatter")
}

/// Get the configuration file path
fn get_config_file_path() -> PathBuf {
    get_config_dir().join("config.json")
}
