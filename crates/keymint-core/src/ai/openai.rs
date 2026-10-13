//! OpenAI Chat Completions, also used for OpenRouter and any compatible server.

use async_trait::async_trait;
use serde_json::{json, Value};

use super::stream::{read_stream, ProgressSink};
use super::{check_status, ensure_ok, non_empty, AiProvider, ChatRequest};
use crate::error::{Error, Result};

pub struct OpenAiCompatible {
    pub base_url: String,
    pub api_key: Option<String>,
    pub http: reqwest::Client,
    /// OpenRouter asks apps to identify themselves.
    pub extra_headers: bool,
}

pub fn build_body(req: &ChatRequest) -> Value {
    json!({
        "model": req.model,
        // Some routers stream by default; ask for one JSON reply.
        "stream": false,
        "messages": req.conversation_with_system(),
    })
}

pub fn parse_response(v: &Value) -> Result<String> {
    let text = v
        .pointer("/choices/0/message/content")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Provider("Unexpected response shape from OpenAI-compatible API".into()))?;
    non_empty(text.to_string())
}

/// Rebuild a chat reply from a server-sent-events body (`data: {...}` lines)
/// for routers that stream even when asked not to.
pub fn sse_to_chat_value(body: &str) -> Option<Value> {
    let mut text = String::new();
    let mut saw_event = false;
    for line in body.lines() {
        let Some(data) = line.trim().strip_prefix("data:") else { continue };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(data) else { continue };
        saw_event = true;
        let piece = event
            .pointer("/choices/0/delta/content")
            .or_else(|| event.pointer("/choices/0/message/content"))
            .and_then(Value::as_str);
        if let Some(piece) = piece {
            text.push_str(piece);
        }
    }
    saw_event.then(|| json!({"choices":[{"message":{"content": text}}]}))
}

/// One line of an SSE stream → the text it carries, if any.
pub fn parse_stream_line(line: &str) -> Result<Option<String>> {
    let Some(data) = line.trim().strip_prefix("data:") else { return Ok(None) };
    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        return Ok(None);
    }
    let Ok(event) = serde_json::from_str::<Value>(data) else { return Ok(None) };
    if let Some(msg) = event.pointer("/error/message").and_then(Value::as_str) {
        return Err(Error::Provider(msg.to_string()));
    }
    Ok(event
        .pointer("/choices/0/delta/content")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string))
}

pub fn parse_models(v: &Value) -> Vec<String> {
    v.get("data")
        .and_then(Value::as_array)
        .map(|models| {
            models.iter().filter_map(|m| m.get("id").and_then(Value::as_str).map(str::to_string)).collect()
        })
        .unwrap_or_default()
}

pub async fn list_models(http: &reqwest::Client, base_url: &str, api_key: Option<&str>) -> Result<Vec<String>> {
    let mut r = http.get(format!("{base_url}/models"));
    if let Some(key) = api_key {
        r = r.bearer_auth(key);
    }
    let resp = r
        .send()
        .await
        .map_err(|e| Error::Provider(format!("Cannot reach {base_url}: {e}")))?;
    Ok(parse_models(&check_status(resp).await?))
}

#[async_trait]
impl AiProvider for OpenAiCompatible {
    async fn complete(&self, req: &ChatRequest) -> Result<String> {
        let mut r = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .json(&build_body(req));
        if let Some(key) = &self.api_key {
            r = r.bearer_auth(key);
        }
        if self.extra_headers {
            r = r.header("X-Title", "Keymint");
        }
        parse_response(&check_status(r.send().await?).await?)
    }

    async fn complete_stream(&self, req: &ChatRequest, sink: &ProgressSink) -> Result<String> {
        let mut body = build_body(req);
        body["stream"] = json!(true);
        let mut r = self.http.post(format!("{}/chat/completions", self.base_url)).json(&body);
        if let Some(key) = &self.api_key {
            r = r.bearer_auth(key);
        }
        if self.extra_headers {
            r = r.header("X-Title", "Keymint");
        }
        let resp = ensure_ok(r.send().await?).await?;
        read_stream(resp, sink, parse_stream_line, parse_response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_has_system_and_user_messages() {
        let body = build_body(&ChatRequest::single("m", "s", "u"));
        assert_eq!(body["model"], "m");
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["content"], "u");
    }

    #[test]
    fn conversation_keeps_order_after_the_system_message() {
        use crate::ai::ChatMessage;
        let req = ChatRequest {
            model: "m".into(),
            system: "s".into(),
            messages: vec![ChatMessage::user("q1"), ChatMessage::assistant("a1"), ChatMessage::user("q2")],
        };
        let roles: Vec<_> = build_body(&req)["messages"].as_array().unwrap().iter().map(|m| m["role"].as_str().unwrap().to_string()).collect();
        assert_eq!(roles, ["system", "user", "assistant", "user"]);
    }

    #[test]
    fn parses_content() {
        let v = json!({"choices":[{"message":{"role":"assistant","content":"  hi \n"}}]});
        assert_eq!(parse_response(&v).unwrap(), "hi");
    }

    #[test]
    fn requests_a_non_streaming_reply() {
        let body = build_body(&ChatRequest::single("m", "s", "u"));
        assert_eq!(body["stream"], false);
    }

    #[test]
    fn rebuilds_text_from_sse() {
        let body = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n\
                    data: {\"choices\":[{\"delta\":{\"content\":\"Hel\"}}]}\n\n\
                    data: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\ndata: [DONE]\n";
        let v = sse_to_chat_value(body).unwrap();
        assert_eq!(parse_response(&v).unwrap(), "Hello");
        assert!(sse_to_chat_value("<html>nope</html>").is_none());
    }

    #[test]
    fn stream_lines() {
        let line = r#"data: {"choices":[{"delta":{"content":"Hi"}}]}"#;
        assert_eq!(parse_stream_line(line).unwrap().as_deref(), Some("Hi"));
        assert_eq!(parse_stream_line("data: [DONE]").unwrap(), None);
        assert_eq!(parse_stream_line(": keep-alive").unwrap(), None);
        assert_eq!(parse_stream_line(r#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#).unwrap(), None);
        assert!(parse_stream_line(r#"data: {"error":{"message":"quota"}}"#).is_err());
    }

    #[test]
    fn parses_model_ids() {
        let v = json!({"object":"list","data":[{"id":"free-best"},{"id":"claude-opus-5"},{"nope":1}]});
        assert_eq!(parse_models(&v), vec!["free-best", "claude-opus-5"]);
        assert!(parse_models(&json!({})).is_empty());
    }

    #[test]
    fn rejects_bad_shape_and_empty() {
        assert!(parse_response(&json!({"nope": 1})).is_err());
        assert!(parse_response(&json!({"choices":[{"message":{"content":"   "}}]})).is_err());
    }
}
