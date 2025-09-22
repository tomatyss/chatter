//! Gemini API client implementation
//!
//! Provides the main client for communicating with Google's Gemini API.

use super::*;
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json;
use std::collections::VecDeque;
use std::time::Duration;

/// Gemini API client
pub struct GeminiClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GeminiClient {
    /// Create a new Gemini client with the given API key
    pub fn new(api_key: String) -> Result<Self> {
        if api_key.trim().is_empty() {
            return Err(anyhow!("API key cannot be empty"));
        }

        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(60))
            .http2_keep_alive_interval(Duration::from_secs(30))
            .http2_keep_alive_timeout(Duration::from_secs(10))
            .http2_keep_alive_while_idle(true)
            .build()?;

        Ok(Self {
            client,
            api_key,
            base_url: GEMINI_API_BASE.to_string(),
        })
    }

    /// Generate content using the specified model
    pub async fn generate_content(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<GenerateContentResponse> {
        let url = format!("{}/models/{}:generateContent", self.base_url, model);

        let response = self
            .client
            .post(&url)
            .query(&[("key", &self.api_key)])
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed: {}", error_text));
        }

        let response_data: GenerateContentResponse = response.json().await?;
        Ok(response_data)
    }

    /// Generate content with streaming response
    pub async fn generate_content_stream(
        &self,
        model: &str,
        request: GenerateContentRequest,
    ) -> Result<std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<String>> + Send>>> {
        let url = format!("{}/models/{}:streamGenerateContent", self.base_url, model);

        let response = self
            .client
            .post(&url)
            .query(&[("alt", "sse"), ("key", &self.api_key)])
            .header("Content-Type", "application/json")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed: {}", error_text));
        }

        // Streaming parser that accumulates across chunks and emits text events
        struct SseParser {
            buffer: String,
            current_event: String,
            queue: VecDeque<String>,
            done: bool,
        }

        impl SseParser {
            fn new() -> Self {
                Self {
                    buffer: String::new(),
                    current_event: String::new(),
                    queue: VecDeque::new(),
                    done: false,
                }
            }

            fn feed(&mut self, chunk: &str) {
                self.buffer.push_str(chunk);
                while let Some(pos) = self.buffer.find('\n') {
                    let mut line = self.buffer[..pos].to_string();
                    // Remove the processed line including the newline
                    self.buffer.drain(..pos + 1);
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        // End of event; try to parse accumulated JSON
                        self.finalize_event();
                        continue;
                    }

                    if let Some(data) = trimmed.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            self.done = true;
                            continue;
                        }
                        self.current_event.push_str(data);
                    } else if trimmed.starts_with("event:")
                        || trimmed.starts_with("id:")
                        || trimmed.starts_with("retry:")
                        || trimmed.starts_with(":")
                    {
                        // ignore control fields and comments
                        continue;
                    } else if trimmed.starts_with('{') {
                        // Some servers may not prefix with data:
                        self.current_event.push_str(trimmed);
                    }
                }
            }

            fn finalize_event(&mut self) {
                let data = self.current_event.trim();
                if !data.is_empty() {
                    if let Ok(response) = serde_json::from_str::<GenerateContentResponse>(data) {
                        if let Some(text) = response.text() {
                            self.queue.push_back(text);
                        }
                    }
                }
                self.current_event.clear();
            }

            fn pop(&mut self) -> Option<String> {
                self.queue.pop_front()
            }

            fn finish(&mut self) {
                // Attempt to parse any remaining event data
                if !self.current_event.trim().is_empty() {
                    self.finalize_event();
                }
            }
        }

        let bytes_stream = response.bytes_stream();
        let stream = futures_util::stream::unfold(
            (bytes_stream, SseParser::new()),
            |(mut bs, mut parser)| async move {
                loop {
                    if let Some(next) = parser.pop() {
                        return Some((Ok(next), (bs, parser)));
                    }

                    match bs.next().await {
                        Some(Ok(bytes)) => {
                            match String::from_utf8(bytes.to_vec()) {
                                Ok(s) => {
                                    parser.feed(&s);
                                    // continue loop to try emit
                                    continue;
                                }
                                Err(e) => {
                                    return Some((
                                        Err(anyhow!("UTF-8 decode error: {}", e)),
                                        (bs, parser),
                                    ));
                                }
                            }
                        }
                        Some(Err(e)) => {
                            if e.is_timeout() {
                                return Some((
                                    Err(anyhow!("Stream timeout: The response took too long")),
                                    (bs, parser),
                                ));
                            } else if e.is_connect() {
                                return Some((
                                    Err(anyhow!("Connection error: Failed to maintain connection")),
                                    (bs, parser),
                                ));
                            } else {
                                return Some((Err(anyhow!("Stream error: {}", e)), (bs, parser)));
                            }
                        }
                        None => {
                            parser.finish();
                            if let Some(next) = parser.pop() {
                                return Some((Ok(next), (bs, parser)));
                            }
                            return None;
                        }
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    /// Send a simple text message and get response
    pub async fn send_message(
        &self,
        model: &str,
        conversation: &[Content],
        system_instruction: Option<&str>,
    ) -> Result<String> {
        let request = build_gemini_request(conversation, system_instruction);

        let response = self.generate_content(model, request).await?;

        response
            .text()
            .ok_or_else(|| anyhow!("No response text received"))
    }

    /// Send a message with streaming response
    pub async fn send_message_stream(
        &self,
        model: &str,
        conversation: &[Content],
        system_instruction: Option<&str>,
    ) -> Result<impl tokio_stream::Stream<Item = Result<String>>> {
        let request = build_gemini_request(conversation, system_instruction);

        self.generate_content_stream(model, request).await
    }
}

fn build_gemini_request(
    conversation: &[Content],
    system_instruction: Option<&str>,
) -> GenerateContentRequest {
    let mut request = GenerateContentRequest::new(normalize_conversation_for_gemini(conversation));

    if let Some(instruction) = system_instruction {
        request = request.with_system_instruction(instruction.to_string());
    }

    request
}

fn normalize_conversation_for_gemini(conversation: &[Content]) -> Vec<Content> {
    conversation
        .iter()
        .filter_map(|content| match content.role.as_str() {
            "user" => Some(Content {
                role: "user".to_string(),
                parts: content.parts.clone(),
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }),
            "model" | "assistant" => Some(Content {
                role: "model".to_string(),
                parts: content.parts.clone(),
                name: None,
                tool_call_id: None,
                tool_calls: Vec::new(),
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_with_role(role: &str, text: &str) -> Content {
        Content {
            role: role.to_string(),
            parts: vec![Part {
                text: text.to_string(),
            }],
            name: None,
            tool_call_id: None,
            tool_calls: Vec::new(),
        }
    }

    #[test]
    fn normalize_conversation_filters_and_maps_roles() {
        let conversation = vec![
            content_with_role("user", "Hello"),
            content_with_role("assistant", "Hi there"),
            content_with_role("system", "Guidance"),
            content_with_role("tool", "Tool output"),
            content_with_role("model", "Response"),
        ];

        let normalized = normalize_conversation_for_gemini(&conversation);

        assert_eq!(
            normalized.len(),
            3,
            "system/tool messages should be dropped"
        );
        assert_eq!(normalized[0].role, "user");
        assert_eq!(normalized[0].parts[0].text, "Hello");
        assert_eq!(normalized[1].role, "model");
        assert_eq!(normalized[1].parts[0].text, "Hi there");
        assert_eq!(normalized[2].role, "model");
        assert_eq!(normalized[2].parts[0].text, "Response");
    }
}
