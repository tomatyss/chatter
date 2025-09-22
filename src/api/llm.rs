use super::client::GeminiClient;
use super::ollama::OllamaClient;
use super::Content;
use anyhow::{anyhow, Result};
use futures_util::Stream;
use std::pin::Pin;

/// Definition of a tool/function exposed to the model
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

/// Model response wrapper used across providers
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub message: Content,
}

/// Unified language model client wrapper
pub enum LlmClient {
    Gemini(GeminiClient),
    Ollama(OllamaClient),
}

impl LlmClient {
    pub fn new_gemini(api_key: String) -> Result<Self> {
        Ok(Self::Gemini(GeminiClient::new(api_key)?))
    }

    pub fn new_ollama(endpoint: String) -> Result<Self> {
        Ok(Self::Ollama(OllamaClient::new(endpoint)?))
    }

    /// Generate a response for the given conversation (non-streaming)
    pub async fn generate(
        &self,
        model: &str,
        conversation: &[Content],
        system_instruction: Option<&str>,
        tools: &[ToolDefinition],
    ) -> Result<ChatResponse> {
        match self {
            LlmClient::Gemini(client) => {
                // Gemini client currently has no tool invocation support
                let response = client
                    .send_message(model, conversation, system_instruction)
                    .await?;
                Ok(ChatResponse {
                    message: Content::model(response),
                })
            }
            LlmClient::Ollama(client) => {
                client
                    .chat(model, conversation, system_instruction, tools)
                    .await
            }
        }
    }

    /// Generate a streaming response for the given conversation
    pub async fn generate_stream(
        &self,
        model: &str,
        conversation: &[Content],
        system_instruction: Option<&str>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        match self {
            LlmClient::Gemini(client) => {
                let stream = client
                    .send_message_stream(model, conversation, system_instruction)
                    .await?;
                Ok(Box::pin(stream) as Pin<Box<dyn Stream<Item = Result<String>> + Send>>)
            }
            LlmClient::Ollama(_) => Err(anyhow!(
                "Streaming responses are not yet supported for Ollama"
            )),
        }
    }
}
