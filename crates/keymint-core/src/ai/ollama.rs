//! Ollama native chat API (local, no key).

use async_trait::async_trait;
use serde_json::{json, Value};

use super::stream::{read_stream, ProgressSink};
use super::{check_status, ensure_ok, non_empty, AiProvider, ChatRequest};
use crate::error::{Error, Result};

pub struct Ollama {
    pub base_url: String,
    pub http: reqwest::Client,
}

pub fn build_body(req: &ChatRequest) -> Value {
    json!({
        "model": req.model,
        "stream": false,
        "messages": req.conversation_with_system(),
    })
}

pub fn parse_response(v: &Value) -> Result<String> {
    let text = v
        .pointer("/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Provider("Unexpected response shape from Ollama".into()))?;
    non_empty(text.to_string())
}

/// One NDJSON line of Ollama's stream → the text it carries, if any.
pub fn parse_stream_line(line: &str) -> Result<Option<String>> {
    let Ok(event) = serde_json::from_str::<Value>(line.trim()) else { return Ok(None) };
    if let Some(msg) = event.get("error").and_then(Value::as_str) {
        return Err(Error::Provider(msg.to_string()));
    }
    Ok(event.pointer("/message/content").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string))
}

pub fn parse_models(v: &Value) -> Vec<String> {
    v.get("models")
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter_map(|m| m.get("name").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub async fn list_models(http: &reqwest::Client, base_url: &str) -> Result<Vec<String>> {
    let resp = http
        .get(format!("{base_url}/api/tags"))
        .send()
        .await
        .map_err(|e| Error::Provider(format!("Cannot reach Ollama at {base_url}: {e}")))?;
    Ok(parse_models(&check_status(resp).await?))
}

#[async_trait]
impl AiProvider for Ollama {
    async fn complete(&self, req: &ChatRequest) -> Result<String> {
        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&build_body(req))
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Cannot reach Ollama at {}: {e}", self.base_url)))?;
        parse_response(&check_status(resp).await?)
    }

    async fn complete_stream(&self, req: &ChatRequest, sink: &ProgressSink) -> Result<String> {
        let mut body = build_body(req);
        body["stream"] = json!(true);
        let resp = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Provider(format!("Cannot reach Ollama at {}: {e}", self.base_url)))?;
        let resp = ensure_ok(resp).await?;
        read_stream(resp, sink, parse_stream_line, parse_response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disables_streaming() {
        let body = build_body(&ChatRequest::single("m", "s", "u"));
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn stream_lines() {
        let line = r#"{"message":{"role":"assistant","content":"ok"},"done":false}"#;
        assert_eq!(parse_stream_line(line).unwrap().as_deref(), Some("ok"));
        assert_eq!(parse_stream_line(r#"{"message":{"content":""},"done":true}"#).unwrap(), None);
        assert!(parse_stream_line(r#"{"error":"model not found"}"#).is_err());
    }

    #[test]
    fn parses_chat_and_models() {
        assert_eq!(parse_response(&json!({"message":{"content":"ok"}})).unwrap(), "ok");
        assert!(parse_response(&json!({})).is_err());
        let tags = json!({"models":[{"name":"llama3.2:latest"},{"name":"qwen2.5:7b"}]});
        assert_eq!(parse_models(&tags), vec!["llama3.2:latest", "qwen2.5:7b"]);
        assert!(parse_models(&json!({})).is_empty());
    }
}
