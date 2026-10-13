//! Anthropic Messages API.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::stream::{read_stream, ProgressSink};
use super::{check_status, ensure_ok, non_empty, AiProvider, ChatRequest};
use crate::error::{Error, Result};

const API_VERSION: &str = "2023-06-01";
const MAX_TOKENS: u32 = 4096;

pub struct Anthropic {
    pub base_url: String,
    pub api_key: Option<String>,
    pub http: reqwest::Client,
}

pub fn build_body(req: &ChatRequest) -> Value {
    json!({
        "model": req.model,
        "max_tokens": MAX_TOKENS,
        "system": req.system,
        "messages": req.conversation(),
    })
}

pub fn parse_response(v: &Value) -> Result<String> {
    let blocks = v
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Provider("Unexpected response shape from Anthropic API".into()))?;
    let text: String = blocks
        .iter()
        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|b| b.get("text").and_then(Value::as_str))
        .collect();
    non_empty(text)
}

/// One line of Anthropic's SSE stream → the text it carries, if any.
pub fn parse_stream_line(line: &str) -> Result<Option<String>> {
    let Some(data) = line.trim().strip_prefix("data:") else { return Ok(None) };
    let Ok(event) = serde_json::from_str::<Value>(data.trim()) else { return Ok(None) };
    match event.get("type").and_then(Value::as_str) {
        Some("content_block_delta") => Ok(event
            .pointer("/delta/text")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)),
        Some("error") => Err(Error::Provider(
            event.pointer("/error/message").and_then(Value::as_str).unwrap_or("Anthropic stream error").to_string(),
        )),
        _ => Ok(None),
    }
}

#[async_trait]
impl AiProvider for Anthropic {
    async fn complete(&self, req: &ChatRequest) -> Result<String> {
        let key = self.api_key.as_deref().unwrap_or_default();
        let r = self
            .http
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", key)
            .header("anthropic-version", API_VERSION)
            .json(&build_body(req));
        parse_response(&check_status(r.send().await?).await?)
    }

    async fn complete_stream(&self, req: &ChatRequest, sink: &ProgressSink) -> Result<String> {
        let key = self.api_key.as_deref().unwrap_or_default();
        let mut body = build_body(req);
        body["stream"] = json!(true);
        let r = self
            .http
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", key)
            .header("anthropic-version", API_VERSION)
            .json(&body);
        let resp = ensure_ok(r.send().await?).await?;
        read_stream(resp, sink, parse_stream_line, parse_response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_is_top_level() {
        let body = build_body(&ChatRequest::single("m", "s", "u"));
        assert_eq!(body["system"], "s");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["max_tokens"], MAX_TOKENS);
    }

    #[test]
    fn multi_turn_messages_alternate_and_system_stays_top_level() {
        use crate::ai::ChatMessage;
        let req = ChatRequest {
            model: "m".into(),
            system: "s".into(),
            messages: vec![ChatMessage::user("q1"), ChatMessage::assistant("a1"), ChatMessage::user("q2")],
        };
        let body = build_body(&req);
        assert_eq!(body["system"], "s");
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[1]["role"], "assistant");
        assert_eq!(msgs[2]["content"], "q2");
    }

    #[test]
    fn joins_text_blocks_and_skips_others() {
        let v = json!({"content":[{"type":"text","text":"Hello "},{"type":"tool_use"},{"type":"text","text":"world"}]});
        assert_eq!(parse_response(&v).unwrap(), "Hello world");
    }

    #[test]
    fn stream_lines() {
        let delta = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Yo"}}"#;
        assert_eq!(parse_stream_line(delta).unwrap().as_deref(), Some("Yo"));
        assert_eq!(parse_stream_line("event: content_block_delta").unwrap(), None);
        assert_eq!(parse_stream_line(r#"data: {"type":"message_start"}"#).unwrap(), None);
        assert!(parse_stream_line(r#"data: {"type":"error","error":{"message":"overloaded"}}"#).is_err());
    }

    #[test]
    fn errors_without_text() {
        assert!(parse_response(&json!({"content":[]})).is_err());
        assert!(parse_response(&json!({})).is_err());
    }
}
