//! CLI module for command-line argument parsing and command definitions

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

pub mod commands;

#[derive(Parser)]
#[command(name = "chatter")]
#[command(about = "A terminal-based chat interface for Google's Gemini AI")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "Your Name <your.email@example.com>")]
pub struct Cli {
    /// Model to use for the conversation
    #[arg(short, long, value_enum)]
    pub model: Option<String>,

    /// System instruction to guide the AI's behavior
    #[arg(short, long)]
    pub system: Option<String>,

    /// Template to use for system instruction
    #[arg(short, long)]
    pub template: Option<String>,

    /// Load a previous chat session
    #[arg(short, long)]
    pub load_session: Option<PathBuf>,

    /// Auto-save the chat session
    #[arg(short, long)]
    pub auto_save: bool,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Send a single query without entering interactive mode
    Query {
        /// The message to send
        message: String,
        /// Model to use for this query
        #[arg(short, long)]
        model: Option<String>,
        /// System instruction for this query
        #[arg(short, long)]
        system: Option<String>,
        /// Template to use for this query
        #[arg(short, long)]
        template: Option<String>,
    },
    /// Template management
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Set the Gemini API key
    SetApiKey,
    /// Show current configuration
    Show,
    /// Reset configuration to defaults
    Reset,
}

#[derive(Subcommand)]
pub enum TemplateAction {
    /// List all available templates
    List {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
        /// Search templates by name or description
        #[arg(short, long)]
        search: Option<String>,
    },
    /// Show details of a specific template
    Show {
        /// Template name
        name: String,
    },
    /// Create a new template
    Create {
        /// Template name
        name: String,
        /// Template description
        #[arg(short, long)]
        description: Option<String>,
        /// Template category
        #[arg(short, long)]
        category: Option<String>,
    },
    /// Edit an existing template
    Edit {
        /// Template name
        name: String,
    },
    /// Delete a template
    Delete {
        /// Template name
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Use a template to start a chat session
    Use {
        /// Template name
        name: String,
        /// Model to use
        #[arg(short, long)]
        model: Option<String>,
    },
}

/// Available Gemini models
#[derive(Clone, Debug, ValueEnum)]
pub enum GeminiModel {
    #[value(name = "gemini-2.5-flash")]
    Flash25,
    #[value(name = "gemini-2.5-pro")]
    Pro25,
    #[value(name = "gemini-1.5-flash")]
    Flash15,
    #[value(name = "gemini-1.5-pro")]
    Pro15,
}

impl GeminiModel {
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            GeminiModel::Flash25 => "gemini-2.5-flash",
            GeminiModel::Pro25 => "gemini-2.5-pro",
            GeminiModel::Flash15 => "gemini-1.5-flash",
            GeminiModel::Pro15 => "gemini-1.5-pro",
        }
    }
}

impl Default for GeminiModel {
    fn default() -> Self {
        GeminiModel::Flash25
    }
}
