//! Runs real HTTP against a local fake server to check streaming behaviour end to end.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use keymint_core::ai::openai::OpenAiCompatible;
use keymint_core::ai::{AiProvider, ChatRequest, Progress, ProgressSink};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Serve one request: send `head`, then each chunk with a pause, then close.
async fn serve(head: &'static str, chunks: Vec<&'static str>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        let mut buf = vec![0u8; 8192];
        let mut seen = Vec::new();
        while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = sock.read(&mut buf).await.unwrap();
            if n == 0 {
                return;
            }
            seen.extend_from_slice(&buf[..n]);
        }
        // Drain the small JSON body if the request has one still in flight.
        tokio::time::sleep(Duration::from_millis(30)).await;
        sock.write_all(head.as_bytes()).await.unwrap();
        for c in chunks {
            sock.write_all(c.as_bytes()).await.unwrap();
            sock.flush().await.unwrap();
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
        let _ = sock.shutdown().await;
    });
    format!("http://{addr}")
}

fn provider(base_url: String) -> OpenAiCompatible {
    OpenAiCompatible { base_url, api_key: Some("k".into()), http: reqwest::Client::new(), extra_headers: false }
}

fn request() -> ChatRequest {
    ChatRequest::single("m", "s", "u")
}

fn collector() -> (ProgressSink, Arc<Mutex<Vec<Progress>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let sink_events = events.clone();
    (Arc::new(move |p| sink_events.lock().unwrap().push(p)), events)
}

const SSE_HEAD: &str = "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\nconnection: close\r\n\r\n";

#[tokio::test]
async fn streams_text_with_stages_in_order_even_when_lines_are_split() {
    let base = serve(
        SSE_HEAD,
        vec![
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hel",
            "lo\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\" wörld\"}}]}\n\n",
            "data: [DONE]\n\n",
        ],
    )
    .await;
    let (sink, events) = collector();
    let text = provider(base).complete_stream(&request(), &sink).await.unwrap();

    assert_eq!(text, "Hello wörld");
    assert_eq!(
        *events.lock().unwrap(),
        vec![
            Progress::Thinking,
            Progress::Writing,
            Progress::Delta { text: "Hello".into() },
            Progress::Delta { text: " wörld".into() },
        ]
    );
}

#[tokio::test]
async fn accepts_a_plain_json_reply_from_a_server_that_ignores_stream() {
    let body = r#"{"choices":[{"message":{"content":"All at once"}}]}"#;
    let base = serve(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n",
        vec![body],
    )
    .await;
    let (sink, events) = collector();
    let text = provider(base).complete_stream(&request(), &sink).await.unwrap();

    assert_eq!(text, "All at once");
    assert!(events.lock().unwrap().contains(&Progress::Delta { text: "All at once".into() }));
}

#[tokio::test]
async fn http_errors_surface_the_providers_message() {
    let base = serve(
        "HTTP/1.1 401 Unauthorized\r\ncontent-type: application/json\r\nconnection: close\r\n\r\n",
        vec![r#"{"error":{"message":"Invalid API key"}}"#],
    )
    .await;
    let (sink, _) = collector();
    let err = provider(base).complete_stream(&request(), &sink).await.unwrap_err().to_string();
    assert!(err.contains("401") && err.contains("Invalid API key"), "{err}");
}

#[tokio::test]
async fn an_error_event_inside_the_stream_fails_the_run() {
    let base = serve(SSE_HEAD, vec!["data: {\"error\":{\"message\":\"quota exceeded\"}}\n\n"]).await;
    let (sink, _) = collector();
    let err = provider(base).complete_stream(&request(), &sink).await.unwrap_err().to_string();
    assert!(err.contains("quota exceeded"), "{err}");
}

#[tokio::test]
async fn an_empty_body_is_reported_clearly() {
    let base = serve(SSE_HEAD, vec![]).await;
    let (sink, _) = collector();
    let err = provider(base).complete_stream(&request(), &sink).await.unwrap_err().to_string();
    assert!(err.contains("empty body"), "{err}");
}
