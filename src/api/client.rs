//! Gemini API client implementation
//! 
//! Provides the main client for communicating with Google's Gemini API.

use super::*;
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json;
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

        let stream = response
            .bytes_stream()
            .map(|chunk_result| {
                match chunk_result {
                    Ok(bytes) => {
                        match String::from_utf8(bytes.to_vec()) {
                            Ok(chunk_str) => {
                                // Parse SSE format with better error handling
                                match parse_sse_chunk_robust(&chunk_str) {
                                    Ok(Some(text)) => Ok(text),
                                    Ok(None) => Ok(String::new()), // Empty chunk is valid for SSE
                                    Err(e) => Err(anyhow!("SSE parsing error: {}", e)),
                                }
                            }
                            Err(e) => Err(anyhow!("UTF-8 decode error: {}", e)),
                        }
                    }
                    Err(e) => {
                        // Categorize the error for better handling
                        if e.is_timeout() {
                            Err(anyhow!("Stream timeout: The response took too long"))
                        } else if e.is_connect() {
                            Err(anyhow!("Connection error: Failed to maintain connection"))
                        } else {
                            Err(anyhow!("Stream error: {}", e))
                        }
                    }
                }
            })
            .filter_map(|result| async move {
                match result {
                    Ok(text) if !text.trim().is_empty() => Some(Ok(text)),
                    Ok(_) => None, // Skip empty chunks
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

/// Parse Server-Sent Events (SSE) chunk with robust error handling
fn parse_sse_chunk_robust(chunk: &str) -> Result<Option<String>> {
    let mut buffer = String::new();
    
    for line in chunk.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        
        // Handle data lines
        if let Some(data) = line.strip_prefix("data: ") {
            // Check for stream end marker
            if data == "[DONE]" {
                return Ok(None);
            }
            
            // Accumulate data (SSE can split JSON across multiple data lines)
            buffer.push_str(data);
            
            // Try to parse accumulated JSON
            if let Ok(response) = serde_json::from_str::<GenerateContentResponse>(&buffer) {
                if let Some(text) = response.text() {
                    return Ok(Some(text));
                }
            }
        }
        
        // Handle event lines (optional)
        else if line.starts_with("event: ") {
            // Could be used for different event types in the future
            continue;
        }
        
        // Handle id lines (optional)
        else if line.starts_with("id: ") {
            // Could be used for event IDs in the future
            continue;
        }
        
        // Handle retry lines (optional)
        else if line.starts_with("retry: ") {
            // Could be used for retry intervals in the future
            continue;
        }
    }
    
    // If we have accumulated data but couldn't parse it, it might be incomplete
    if !buffer.is_empty() {
        return Err(anyhow!("Incomplete or malformed JSON data: {}", buffer));
    }
    
    // Empty chunk - this is normal for SSE keep-alive
    Ok(None)
}
