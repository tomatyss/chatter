use super::{Content, ModelToolCall, Part, CONNECT_TIMEOUT, REQUEST_TIMEOUT};
use crate::api::llm::{ChatResponse, ToolDefinition};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{self, Value};
use std::time::Duration;

/// HTTP client for interacting with an Ollama server
pub struct OllamaClient {
    client: Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(endpoint: String) -> Result<Self> {
        let trimmed = endpoint.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Ollama endpoint cannot be empty"));
        }

        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(60))
            .build()?;

        Ok(Self {
            client,
            base_url: trimmed.trim_end_matches('/').to_string(),
        })
    }

    pub async fn chat(
        &self,
        model: &str,
        conversation: &[Content],
        system_instruction: Option<&str>,
        tools: &[ToolDefinition],
    ) -> Result<ChatResponse> {
        let mut messages = Vec::new();

        if let Some(system) = system_instruction {
            if !system.trim().is_empty() {
                messages.push(OllamaMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                    name: None,
                    tool_call_id: None,
                    tool_calls: None,
                });
            }
        }

        for content in conversation {
            messages.push(convert_content_to_ollama_message(content));
        }

        let request = OllamaChatRequest {
            model,
            messages,
            stream: false,
            tools: if tools.is_empty() {
                None
            } else {
                Some(
                    tools
                        .iter()
                        .map(|tool| OllamaTool {
                            kind: "function".to_string(),
                            function: OllamaToolFunction {
                                name: tool.name.clone(),
                                description: tool.description.clone(),
                                parameters: tool.parameters.clone(),
                            },
                        })
                        .collect(),
                )
            },
        };

        let url = format!("{}/api/chat", self.base_url);

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let bytes = response.bytes().await?;

        if !status.is_success() {
            let error_text = String::from_utf8_lossy(&bytes);
            return Err(anyhow!("Ollama request failed: {}", error_text));
        }

        let response: OllamaChatResponse = serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "Failed to decode Ollama response body: {}",
                String::from_utf8_lossy(&bytes)
            )
        })?;
        let message = response.message;

        let mut tool_calls = Vec::new();
        for call in message.tool_calls.unwrap_or_default() {
            if let Some(kind) = &call.kind {
                if kind != "function" {
                    continue;
                }
            }
            let arguments = call.function.arguments;
            tool_calls.push(ModelToolCall {
                id: call.id,
                name: call.function.name,
                arguments,
            });
        }

        let mut parts = Vec::new();
        if let Some(text) = message.content {
            if !text.is_empty() {
                parts.push(Part { text });
            }
        }

        let mut content = if parts.is_empty() {
            Content {
                role: "model".to_string(),
                parts: vec![Part {
                    text: String::new(),
                }],
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }
        } else {
            Content {
                role: "model".to_string(),
                parts,
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }
        };

        content.tool_calls = tool_calls;

        Ok(ChatResponse { message: content })
    }
}

fn convert_content_to_ollama_message(content: &Content) -> OllamaMessage {
    let role = match content.role.as_str() {
        "user" => "user",
        "tool" => "tool",
        "assistant" => "assistant",
        "system" => "system",
        _ => "assistant",
    }
    .to_string();

    let mut message = OllamaMessage {
        role,
        content: content
            .parts
            .first()
            .map(|p| p.text.clone())
            .unwrap_or_default(),
        name: content.name.clone(),
        tool_call_id: content.tool_call_id.clone(),
        tool_calls: None,
    };

    if !content.tool_calls.is_empty() {
        let calls = content
            .tool_calls
            .iter()
            .map(|call| OllamaMessageToolCall {
                kind: "function".to_string(),
                id: call.id.clone(),
                function: OllamaToolFunctionCall {
                    name: call.name.clone(),
                    arguments: call.arguments.clone(),
                },
            })
            .collect();
        message.tool_calls = Some(calls);
    }

    // Flatten tool call markers stored using role prefixes such as "tool:read_file"
    if message.role == "assistant" && content.role.starts_with("tool:") {
        message.role = "tool".to_string();
        message.name = Some(content.role[5..].to_string());
    }

    // Ensure tool role messages include a name if encoded in content.role
    if message.role == "tool" && message.name.is_none() {
        if let Some(name) = &content.name {
            message.name = Some(name.clone());
        } else if let Some(prefix_name) = content.role.strip_prefix("tool:").map(|s| s.to_string())
        {
            message.name = Some(prefix_name);
        }
    }

    message
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaMessageToolCall>>,
}

#[derive(Debug, Serialize)]
struct OllamaMessageToolCall {
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    function: OllamaToolFunctionCall,
}

#[derive(Debug, Serialize)]
struct OllamaToolFunctionCall {
    name: String,
    arguments: Value,
}

#[derive(Debug, Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    kind: String,
    function: OllamaToolFunction,
}

#[derive(Debug, Serialize)]
struct OllamaToolFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    #[serde(rename = "role")]
    _role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaResponseToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseToolCall {
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    function: OllamaResponseFunction,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseFunction {
    name: String,
    #[serde(deserialize_with = "deserialize_arguments")]
    arguments: Value,
}

fn deserialize_arguments<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    let raw: Value = Value::deserialize(deserializer)?;
    match raw {
        Value::String(s) => {
            if s.trim().is_empty() {
                Ok(Value::Object(serde_json::Map::new()))
            } else {
                match serde_json::from_str::<Value>(&s) {
                    Ok(parsed) => Ok(parsed),
                    Err(_) => Ok(Value::String(s)),
                }
            }
        }
        other => Ok(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tool_call_without_type_field() {
        let payload = r#"{
            "model": "qwen3",
            "created_at": "2025-09-22T14:42:33.34871Z",
            "message": {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "function": {
                            "name": "read_file",
                            "arguments": {"path": "Cargo.toml"}
                        }
                    }
                ]
            },
            "done": true,
            "done_reason": "stop"
        }"#;

        let response: OllamaChatResponse = serde_json::from_str(payload).unwrap();
        let calls = response.message.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function.name, "read_file");
        assert_eq!(calls[0].function.arguments["path"], "Cargo.toml");
    }
}
