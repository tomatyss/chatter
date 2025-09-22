//! Gemini API client module
//!
//! Handles communication with Google's Gemini API, including request/response
//! serialization, streaming, and error handling.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

pub mod client;
pub mod llm;
pub mod models;
pub mod ollama;
pub mod streaming;

pub use llm::{LlmClient, ToolDefinition};

/// Base URL for the Gemini API
const GEMINI_API_BASE: &str = "https://generativelanguage.googleapis.com/v1beta";

/// HTTP client configuration
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes for streaming responses
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30); // 30 seconds to establish connection

/// Content part in a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub text: String,
}

/// Message content with role and parts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub tool_calls: Vec<ModelToolCall>,
}

/// Model tool call representation used across providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

/// System instruction for the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInstruction {
    pub parts: Vec<Part>,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i32>,
}

/// Request to generate content
#[derive(Debug, Clone, Serialize)]
pub struct GenerateContentRequest {
    pub contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

/// Candidate response from the model
#[derive(Debug, Clone, Deserialize)]
pub struct Candidate {
    pub content: Content,
    #[serde(rename = "finishReason")]
    #[allow(dead_code)]
    pub finish_reason: Option<String>,
}

/// Response from the generate content API
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
}

impl GenerateContentRequest {
    /// Create a new request with the given contents
    pub fn new(contents: Vec<Content>) -> Self {
        Self {
            contents,
            system_instruction: None,
            generation_config: None,
        }
    }

    /// Add system instruction to the request
    pub fn with_system_instruction(mut self, instruction: String) -> Self {
        self.system_instruction = Some(SystemInstruction {
            parts: vec![Part { text: instruction }],
        });
        self
    }

    /// Add generation configuration
    #[allow(dead_code)]
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }
}

impl Content {
    /// Create user content with text
    pub fn user(text: String) -> Self {
        Self {
            role: "user".to_string(),
            parts: vec![Part { text }],
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    /// Create model content with text
    pub fn model(text: String) -> Self {
        Self {
            role: "model".to_string(),
            parts: vec![Part { text }],
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }
}

impl GenerateContentResponse {
    /// Get the text from the first candidate
    pub fn text(&self) -> Option<String> {
        self.candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
    }
}
