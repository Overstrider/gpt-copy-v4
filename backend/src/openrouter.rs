use std::{pin::Pin, time::Duration};

use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<String, ChatProviderError>> + Send>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

#[async_trait]
pub trait ChatProvider: Send + Sync {
    async fn complete(&self, messages: Vec<ProviderMessage>) -> Result<String, ChatProviderError>;

    async fn stream(
        &self,
        messages: Vec<ProviderMessage>,
    ) -> Result<ProviderStream, ChatProviderError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ChatProviderError {
    #[error("OPENROUTER_API_KEY is required for chat requests")]
    MissingApiKey,
    #[error("OpenRouter returned {status}: {body}")]
    Upstream { status: StatusCode, body: String },
    #[error("request failed: {0}")]
    Request(String),
    #[error("invalid provider response: {0}")]
    InvalidResponse(String),
}

impl ChatProviderError {
    pub fn public_message(&self) -> String {
        match self {
            Self::MissingApiKey => "OPENROUTER_API_KEY is not configured".to_string(),
            Self::Upstream { status, .. } => format!("OpenRouter request failed with {status}"),
            Self::Request(_) => "OpenRouter request failed".to_string(),
            Self::InvalidResponse(_) => "OpenRouter response was invalid".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct OpenRouterClient {
    client: Client,
    api_key: Option<String>,
    model: String,
    base_url: String,
}

impl OpenRouterClient {
    pub fn new(api_key: Option<String>, model: String, base_url: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .expect("reqwest client"),
            api_key,
            model,
            base_url,
        }
    }

    fn api_key(&self) -> Result<&str, ChatProviderError> {
        self.api_key
            .as_deref()
            .ok_or(ChatProviderError::MissingApiKey)
    }

    async fn send(
        &self,
        request: OpenRouterRequest,
    ) -> Result<reqwest::Response, ChatProviderError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .bearer_auth(self.api_key()?)
            .header("HTTP-Referer", "http://localhost:3000")
            .header("X-Title", "gpt-copy-v4")
            .json(&request)
            .send()
            .await
            .map_err(|error| ChatProviderError::Request(error.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "".to_string());
            return Err(ChatProviderError::Upstream { status, body });
        }

        Ok(response)
    }
}

#[async_trait]
impl ChatProvider for OpenRouterClient {
    async fn complete(&self, messages: Vec<ProviderMessage>) -> Result<String, ChatProviderError> {
        let response = self
            .send(OpenRouterRequest {
                model: self.model.clone(),
                messages,
                stream: false,
            })
            .await?;
        let body = response
            .json::<OpenRouterResponse>()
            .await
            .map_err(|error| ChatProviderError::InvalidResponse(error.to_string()))?;

        body.choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| {
                ChatProviderError::InvalidResponse("missing assistant content".to_string())
            })
    }

    async fn stream(
        &self,
        messages: Vec<ProviderMessage>,
    ) -> Result<ProviderStream, ChatProviderError> {
        let response = self
            .send(OpenRouterRequest {
                model: self.model.clone(),
                messages,
                stream: true,
            })
            .await?;
        let bytes = response
            .bytes_stream()
            .map(|next| next.map_err(|error| ChatProviderError::Request(error.to_string())));

        Ok(decode_sse_stream(bytes))
    }
}

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<ProviderMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamResponse {
    choices: Vec<OpenRouterStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamChoice {
    delta: OpenRouterStreamDelta,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamDelta {
    content: Option<String>,
}

fn parse_stream_line(line: &str) -> Result<Option<String>, ChatProviderError> {
    let Some(data) = line.strip_prefix("data:") else {
        return Ok(None);
    };
    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        return Ok(None);
    }

    let parsed = serde_json::from_str::<OpenRouterStreamResponse>(data)
        .map_err(|error| ChatProviderError::InvalidResponse(error.to_string()))?;
    Ok(parsed
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.delta.content)
        .filter(|content| !content.is_empty()))
}

fn decode_sse_stream<S>(bytes: S) -> ProviderStream
where
    S: Stream<Item = Result<Bytes, ChatProviderError>> + Send + 'static,
{
    let stream = async_stream::try_stream! {
        let mut buffer = Vec::new();
        tokio::pin!(bytes);

        while let Some(next) = bytes.next().await {
            let chunk = next?;
            buffer.extend_from_slice(&chunk);

            while let Some(index) = buffer.iter().position(|byte| *byte == b'\n') {
                let mut line_bytes = buffer.drain(..=index).collect::<Vec<_>>();
                line_bytes.pop();
                let line = std::str::from_utf8(&line_bytes)
                    .map_err(|error| ChatProviderError::InvalidResponse(error.to_string()))?;

                if let Some(content) = parse_stream_line(line.trim())? {
                    yield content;
                }
            }
        }

        if !buffer.is_empty() {
            let remaining = std::str::from_utf8(&buffer)
                .map_err(|error| ChatProviderError::InvalidResponse(error.to_string()))?
                .trim();
            if !remaining.is_empty() {
                match parse_stream_line(remaining)? {
                    Some(content) => yield content,
                    None => {}
                }
            }
        }
    };

    Box::pin(stream)
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures_util::{TryStreamExt, stream};

    use super::*;

    #[tokio::test]
    async fn stream_decoder_accepts_utf8_split_across_network_chunks() {
        let line = r#"data: {"choices":[{"delta":{"content":"💡"}}]}"#;
        let bytes = format!("{line}\n\n").into_bytes();
        let emoji_start = bytes
            .windows("💡".len())
            .position(|window| window == "💡".as_bytes())
            .expect("emoji bytes");

        let chunks = vec![
            Ok(Bytes::copy_from_slice(&bytes[..emoji_start + 1])),
            Ok(Bytes::copy_from_slice(&bytes[emoji_start + 1..])),
        ];

        let decoded = decode_sse_stream(stream::iter(chunks))
            .try_collect::<Vec<_>>()
            .await
            .expect("valid split utf-8 stream");

        assert_eq!(decoded, vec!["💡".to_string()]);
    }
}
