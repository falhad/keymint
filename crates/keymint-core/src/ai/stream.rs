//! Streaming plumbing shared by all providers: progress events, a UTF-8-safe
//! line splitter and the loop that turns a response body into text.

use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result};

/// What the AI side is doing, for the progress screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Progress {
    /// Request is being sent.
    Connecting { provider: String, model: String },
    /// Request accepted; waiting for the first token.
    Thinking,
    /// Text has started to arrive.
    Writing,
    /// A piece of the reply.
    Delta { text: String },
}

pub type ProgressSink = Arc<dyn Fn(Progress) + Send + Sync>;

pub fn no_progress() -> ProgressSink {
    Arc::new(|_| {})
}

/// Parses one line of a streaming body. `Ok(None)` means "nothing to show".
pub type LineParser = fn(&str) -> Result<Option<String>>;
/// Reads a complete, non-streamed reply (for servers that ignore `stream: true`).
pub type ReplyParser = fn(&Value) -> Result<String>;

/// Splits bytes into lines without breaking multi-byte characters that
/// straddle chunk boundaries.
#[derive(Default)]
pub struct LineBuffer(Vec<u8>);

impl LineBuffer {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.0.extend_from_slice(chunk);
        let mut lines = Vec::new();
        while let Some(pos) = self.0.iter().position(|&b| b == b'\n') {
            let mut line: Vec<u8> = self.0.drain(..=pos).collect();
            line.pop(); // '\n'
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            lines.push(String::from_utf8_lossy(&line).into_owned());
        }
        lines
    }

    /// A final line without a trailing newline.
    pub fn finish(&mut self) -> Option<String> {
        if self.0.is_empty() {
            return None;
        }
        let line = String::from_utf8_lossy(&self.0).into_owned();
        self.0.clear();
        Some(line)
    }
}

/// Keep at most this much of a body while waiting to learn whether it streams.
const MAX_RAW: usize = 1 << 20;

struct Accumulator {
    full: String,
    writing: bool,
    raw: Vec<u8>,
}

fn feed(line: &str, parse: LineParser, sink: &ProgressSink, acc: &mut Accumulator) -> Result<()> {
    if let Some(piece) = parse(line)? {
        if !acc.writing {
            acc.writing = true;
            acc.raw = Vec::new();
            sink(Progress::Writing);
        }
        acc.full.push_str(&piece);
        sink(Progress::Delta { text: piece });
    }
    Ok(())
}

/// Consume a streaming response, reporting progress, and return the full text.
pub async fn read_stream(
    mut resp: reqwest::Response,
    sink: &ProgressSink,
    parse: LineParser,
    fallback: ReplyParser,
) -> Result<String> {
    sink(Progress::Thinking);
    let mut lines = LineBuffer::default();
    let mut acc = Accumulator { full: String::new(), writing: false, raw: Vec::new() };

    while let Some(chunk) = resp.chunk().await? {
        if !acc.writing && acc.raw.len() < MAX_RAW {
            acc.raw.extend_from_slice(&chunk);
        }
        for line in lines.push(&chunk) {
            feed(&line, parse, sink, &mut acc)?;
        }
    }
    if let Some(line) = lines.finish() {
        feed(&line, parse, sink, &mut acc)?;
    }

    if acc.full.trim().is_empty() {
        // Not a stream after all: maybe one plain JSON reply.
        return match serde_json::from_slice::<Value>(&acc.raw) {
            Ok(v) => {
                let text = fallback(&v)?;
                sink(Progress::Writing);
                sink(Progress::Delta { text: text.clone() });
                Ok(text)
            }
            Err(_) => {
                let snippet: String = String::from_utf8_lossy(&acc.raw).trim().chars().take(160).collect();
                Err(Error::Provider(format!(
                    "The provider sent no text: {}",
                    if snippet.is_empty() { "<empty body>" } else { &snippet }
                )))
            }
        };
    }
    super::non_empty(acc.full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_lines_across_chunks() {
        let mut b = LineBuffer::default();
        assert!(b.push(b"data: {\"a\"").is_empty());
        assert_eq!(b.push(b":1}\r\n\ndata: x"), vec!["data: {\"a\":1}", ""]);
        assert_eq!(b.finish().as_deref(), Some("data: x"));
        assert_eq!(b.finish(), None);
    }

    #[test]
    fn keeps_multibyte_characters_intact_across_chunks() {
        let bytes = "héllo wörld\n".as_bytes();
        let split = bytes.iter().position(|&b| b == 0xC3).unwrap() + 1; // inside "é"
        let mut b = LineBuffer::default();
        assert!(b.push(&bytes[..split]).is_empty());
        assert_eq!(b.push(&bytes[split..]), vec!["héllo wörld"]);
    }
}
