//! Gemini API client implementation
//! 
//! Provides the main client for communicating with Google's Gemini API.

use super::*;
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json;

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
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("API request failed: {}", error_text));
        }

        let stream = response
            .bytes_stream()
            .map(|chunk| {
                chunk
                    .map_err(|e| anyhow!("Stream error: {}", e))
                    .and_then(|bytes| {
                        String::from_utf8(bytes.to_vec())
                            .map_err(|e| anyhow!("UTF-8 decode error: {}", e))
                    })
            })
            .filter_map(|result| async move {
                match result {
                    Ok(chunk) => {
                        // Parse SSE format
                        if let Some(text) = parse_sse_chunk(&chunk) {
                            Some(Ok(text))
                        } else {
                            None
                        }
                    }
                    Err(e) => Some(Err(e)),
                }
            });

        Ok(Box::pin(stream))
    }

    /// Send a simple text message and get response
    pub async fn send_message(
        &self,
        model: &str,
        message: &str,
        history: &[Content],
        system_instruction: Option<&str>,
    ) -> Result<String> {
        let mut contents = history.to_vec();
        contents.push(Content::user(message.to_string()));

        let mut request = GenerateContentRequest::new(contents);
        
        if let Some(instruction) = system_instruction {
            request = request.with_system_instruction(instruction.to_string());
        }

        let response = self.generate_content(model, request).await?;
        
        response
            .text()
            .ok_or_else(|| anyhow!("No response text received"))
    }

    /// Send a message with streaming response
    pub async fn send_message_stream(
        &self,
        model: &str,
        message: &str,
        history: &[Content],
        system_instruction: Option<&str>,
    ) -> Result<impl tokio_stream::Stream<Item = Result<String>>> {
        let mut contents = history.to_vec();
        contents.push(Content::user(message.to_string()));

        let mut request = GenerateContentRequest::new(contents);
        
        if let Some(instruction) = system_instruction {
            request = request.with_system_instruction(instruction.to_string());
        }

        self.generate_content_stream(model, request).await
    }
}

/// Parse Server-Sent Events (SSE) chunk to extract text content
fn parse_sse_chunk(chunk: &str) -> Option<String> {
    for line in chunk.lines() {
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                return None;
            }
            
            if let Ok(response) = serde_json::from_str::<GenerateContentResponse>(data) {
                if let Some(text) = response.text() {
                    return Some(text);
                }
            }
        }
    }
    None
}
