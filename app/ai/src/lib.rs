//! AI provider layer (Phase 7: P7-01, P7-02, P7-03 / EDGE_CASES §11).
//!
//! Providers: None (default), Local (Ollama/LM Studio/OpenAI-compatible),
//! Cloud (OpenAI-compatible & Anthropic), and Odysseus (ported from Odyssey).

mod agent;
pub use agent::*;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    #[default]
    None,
    Local,
    Cloud,
    Odysseus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiConfig {
    pub provider: ProviderKind,
    pub endpoint: String,
    pub model: String,
    /// A model that can look at pictures (for screenshots). Empty = use `model`.
    #[serde(default)]
    pub vision_model: String,
    #[serde(default, skip_serializing)]
    pub api_key: Option<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: ProviderKind::None,
            endpoint: String::new(),
            model: String::new(),
            vision_model: String::new(),
            api_key: None,
        }
    }
}

/// Settings as communicated to the UI (keys never leave Rust, EDGE_CASES §11).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiSettingsUi {
    pub provider: ProviderKind,
    pub endpoint: String,
    pub model: String,
    pub vision_model: String,
    pub has_key: bool,
}

impl From<&AiConfig> for AiSettingsUi {
    fn from(c: &AiConfig) -> Self {
        Self {
            provider: c.provider,
            endpoint: c.endpoint.clone(),
            model: c.model.clone(),
            vision_model: c.vision_model.clone(),
            has_key: c.api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data", rename_all = "kebab-case")]
pub enum AiEvent {
    Delta(String),
    Source {
        title: String,
        url: String,
    },
    /// What the assistant is doing right now ("Searching the web for ...").
    Step(String),
    /// Something for the browser to do for the assistant (open a tab).
    Action {
        kind: String,
        url: String,
    },
    Done,
    Error(String),
}

/// `llama3` matches `llama3:latest` (Ollama adds a tag).
pub fn model_installed(models: &[String], wanted: &str) -> bool {
    models
        .iter()
        .any(|m| m == wanted || m.strip_suffix(":latest") == Some(wanted))
}

pub fn missing_model_message(model: &str) -> String {
    format!("Model {model} isn't installed. Run `ollama pull {model}` or pick another model.")
}

/// Root of an Ollama-style server: no trailing slash and no `/v1` (so `/api/tags` and
/// `/v1/chat/completions` both work whichever form the user typed).
pub fn local_root(endpoint: &str) -> String {
    let t = endpoint.trim().trim_end_matches('/');
    let t = t.strip_suffix("/v1").unwrap_or(t);
    if t.is_empty() {
        "http://127.0.0.1:11434".to_string()
    } else {
        t.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct AiClient {
    http: reqwest::Client,
}

impl Default for AiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AiClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Constellations/0.1")
            .build()
            .unwrap_or_default();
        Self { http }
    }

    /// Test whether the configured provider is reachable and credentials are valid.
    pub async fn test_connection(&self, config: &AiConfig) -> Result<String, String> {
        match config.provider {
            ProviderKind::None => Ok("AI is disabled.".to_string()),
            ProviderKind::Local => {
                let base = local_root(&config.endpoint);
                let url = format!("{base}/api/tags");
                let resp = self
                    .http
                    .get(&url)
                    .timeout(Duration::from_secs(5))
                    .send()
                    .await
                    .map_err(|e| format!("Cannot reach local AI at {base}: {e}"))?;
                if !resp.status().is_success() {
                    return Err(format!("Local AI returned status {}", resp.status()));
                }
                let body: Value = resp.json().await.unwrap_or(Value::Null);
                let models: Vec<String> = body["models"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|m| m["name"].as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let wanted = config.model.trim();
                if !wanted.is_empty() && !model_installed(&models, wanted) {
                    return Err(missing_model_message(wanted));
                }
                if models.is_empty() {
                    Ok("Connected to local AI, but no models are installed yet.".to_string())
                } else {
                    Ok(format!(
                        "Connected to local AI. Models: {}",
                        models.join(", ")
                    ))
                }
            }
            ProviderKind::Cloud => {
                if config
                    .api_key
                    .as_ref()
                    .map(|k| k.trim().is_empty())
                    .unwrap_or(true)
                {
                    return Err("API key is missing.".to_string());
                }
                let base = if config.endpoint.trim().is_empty() {
                    "https://api.openai.com/v1"
                } else {
                    config.endpoint.trim().trim_end_matches('/')
                };
                let key = config.api_key.as_deref().unwrap_or_default();
                let url = format!("{base}/models");
                let resp = self
                    .http
                    .get(&url)
                    .bearer_auth(key)
                    .timeout(Duration::from_secs(8))
                    .send()
                    .await
                    .map_err(|e| format!("Cannot reach cloud provider at {base}: {e}"))?;
                let status = resp.status();
                if status.is_success() {
                    Ok("Cloud provider connected.".to_string())
                } else if status.as_u16() == 401 {
                    Err("Invalid API key (401 Unauthorized).".to_string())
                } else if status.as_u16() == 429 {
                    Err("Rate limited (429 Too Many Requests).".to_string())
                } else {
                    Err(format!("Cloud provider returned status {status}"))
                }
            }
            ProviderKind::Odysseus => {
                let base = config.endpoint.trim().trim_end_matches('/');
                if base.is_empty() {
                    return Err("Odysseus address is not configured.".to_string());
                }
                let key = config.api_key.as_deref().unwrap_or_default();
                let url = format!("{base}/api/tokens/self");
                let resp = self
                    .http
                    .get(&url)
                    .bearer_auth(key)
                    .timeout(Duration::from_secs(8))
                    .send()
                    .await
                    .map_err(|e| format!("Cannot reach Odysseus at {base}: {e}"))?;
                let status = resp.status();
                if status.is_success() {
                    Ok("Connected to Odysseus.".to_string())
                } else if status.as_u16() == 401 {
                    Err("Odysseus token rejected. Please reconnect in Settings.".to_string())
                } else {
                    Err(format!("Odysseus returned status {status}"))
                }
            }
        }
    }

    /// The models an Ollama-style local server has installed (empty for other providers).
    pub async fn list_local_models(&self, config: &AiConfig) -> Result<Vec<String>, String> {
        if config.provider != ProviderKind::Local {
            return Ok(Vec::new());
        }
        let url = format!("{}/api/tags", local_root(&config.endpoint));
        let resp = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| format!("Cannot reach the local AI at {url}: {e}"))?;
        let body: Value = resp.json().await.map_err(|e| e.to_string())?;
        Ok(body["models"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|m| m["name"].as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Ask a question and stream response events to `sink`.
    pub async fn ask(
        &self,
        config: &AiConfig,
        prompt: &str,
        context: Option<&str>,
        cancelled: Arc<AtomicBool>,
        sink: impl Fn(AiEvent) + Send + 'static,
    ) {
        if config.provider == ProviderKind::None {
            sink(AiEvent::Error(
                "No AI provider configured. Open Settings -> AI to choose one.".to_string(),
            ));
            sink(AiEvent::Done);
            return;
        }

        // Limit reference context to 12,000 chars to avoid model context overflow (EDGE_CASES §11)
        let clean_context = context.map(|c| {
            if c.len() > 12_000 {
                let mut s = c.chars().take(12_000).collect::<String>();
                s.push_str("\n... [reference text truncated]");
                s
            } else {
                c.to_string()
            }
        });

        match config.provider {
            ProviderKind::None => unreachable!(),
            ProviderKind::Local => {
                self.stream_openai_compatible(
                    &format!("{}/v1", local_root(&config.endpoint)),
                    if config.model.trim().is_empty() {
                        "llama3"
                    } else {
                        &config.model
                    },
                    "",
                    prompt,
                    clean_context.as_deref(),
                    cancelled,
                    sink,
                )
                .await;
            }
            ProviderKind::Cloud => {
                let key = config.api_key.as_deref().unwrap_or_default();
                let base = if config.endpoint.trim().is_empty() {
                    "https://api.openai.com/v1"
                } else {
                    config.endpoint.trim().trim_end_matches('/')
                };
                let model = if config.model.trim().is_empty() {
                    if base.contains("anthropic") {
                        "claude-3-5-sonnet-20241022"
                    } else {
                        "gpt-4o-mini"
                    }
                } else {
                    &config.model
                };
                if base.contains("anthropic") {
                    self.stream_anthropic(
                        base,
                        model,
                        key,
                        prompt,
                        clean_context.as_deref(),
                        cancelled,
                        sink,
                    )
                    .await;
                } else {
                    self.stream_openai_compatible(
                        base,
                        model,
                        key,
                        prompt,
                        clean_context.as_deref(),
                        cancelled,
                        sink,
                    )
                    .await;
                }
            }
            ProviderKind::Odysseus => {
                self.stream_odysseus(config, prompt, clean_context.as_deref(), cancelled, sink)
                    .await;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn stream_anthropic(
        &self,
        base_url: &str,
        model: &str,
        key: &str,
        prompt: &str,
        context: Option<&str>,
        cancelled: Arc<AtomicBool>,
        sink: impl Fn(AiEvent) + Send + 'static,
    ) {
        let endpoint = if base_url.ends_with("/messages") {
            base_url.to_string()
        } else {
            format!("{base_url}/v1/messages")
        };

        let system_prompt = if let Some(ctx) = context {
            format!(
                "You are an AI browser assistant. Use the following page context as reference:\n\n{ctx}"
            )
        } else {
            "You are Constellations browser assistant. Answer concisely and clearly.".to_string()
        };

        let payload = json!({
            "model": model,
            "max_tokens": 1024,
            "system": system_prompt,
            "messages": [
                { "role": "user", "content": prompt }
            ],
            "stream": true
        });

        let resp = match self
            .http
            .post(&endpoint)
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                sink(AiEvent::Error(format!("Connection error: {e}")));
                sink(AiEvent::Done);
                return;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            sink(AiEvent::Error(format!("Anthropic responded with {status}")));
            sink(AiEvent::Done);
            return;
        }

        let mut stream = resp.bytes_stream();
        let mut buf = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            if cancelled.load(Ordering::Relaxed) {
                break;
            }
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    sink(AiEvent::Error(format!("Stream interrupted: {e}")));
                    break;
                }
            };
            buf.extend_from_slice(&chunk);

            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let line_str = String::from_utf8_lossy(&line).trim().to_string();
                let Some(data) = line_str.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if let Ok(val) = serde_json::from_str::<Value>(data) {
                    if val["type"] == "content_block_delta" {
                        if let Some(text) = val["delta"]["text"].as_str().filter(|t| !t.is_empty())
                        {
                            sink(AiEvent::Delta(text.to_string()));
                        }
                    } else if val["type"] == "message_stop" {
                        break;
                    }
                }
            }
        }

        sink(AiEvent::Done);
    }

    #[allow(clippy::too_many_arguments)]
    async fn stream_openai_compatible(
        &self,
        base_url: &str,
        model: &str,
        key: &str,
        prompt: &str,
        context: Option<&str>,
        cancelled: Arc<AtomicBool>,
        sink: impl Fn(AiEvent) + Send + 'static,
    ) {
        let endpoint = if base_url.ends_with("/chat/completions") {
            base_url.to_string()
        } else {
            format!("{base_url}/chat/completions")
        };

        let mut messages = Vec::new();
        if let Some(ctx) = context {
            messages.push(json!({
                "role": "system",
                "content": format!("You are an AI browser assistant. Use the following page context as reference:\n\n{ctx}")
            }));
        } else {
            messages.push(json!({
                "role": "system",
                "content": "You are Constellations browser assistant. Answer concisely and clearly."
            }));
        }
        messages.push(json!({
            "role": "user",
            "content": prompt
        }));

        let payload = json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "temperature": 0.3
        });

        let mut req = self.http.post(&endpoint).json(&payload);
        if !key.is_empty() {
            req = req.bearer_auth(key);
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                sink(AiEvent::Error(format!("Connection error: {e}")));
                sink(AiEvent::Done);
                return;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let msg = if status == reqwest::StatusCode::NOT_FOUND && base_url.contains("11434") {
                missing_model_message(model)
            } else {
                format!("Provider responded with {status}")
            };
            sink(AiEvent::Error(msg));
            sink(AiEvent::Done);
            return;
        }

        let mut stream = resp.bytes_stream();
        let mut buf = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            if cancelled.load(Ordering::Relaxed) {
                break;
            }
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    sink(AiEvent::Error(format!("Stream interrupted: {e}")));
                    break;
                }
            };
            buf.extend_from_slice(&chunk);

            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let line_str = String::from_utf8_lossy(&line).trim().to_string();
                let Some(data) = line_str.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    break;
                }
                if let Ok(val) = serde_json::from_str::<Value>(data)
                    && let Some(delta) = val["choices"][0]["delta"]["content"]
                        .as_str()
                        .filter(|d| !d.is_empty())
                {
                    sink(AiEvent::Delta(delta.to_string()));
                }
            }
        }

        sink(AiEvent::Done);
    }

    async fn stream_odysseus(
        &self,
        config: &AiConfig,
        prompt: &str,
        context: Option<&str>,
        cancelled: Arc<AtomicBool>,
        sink: impl Fn(AiEvent) + Send + 'static,
    ) {
        let base = config.endpoint.trim().trim_end_matches('/');
        let key = config.api_key.as_deref().unwrap_or_default();
        let mut form = reqwest::multipart::Form::new()
            .text("message", prompt.to_string())
            .text("mode", "chat");

        if let Some(ctx) = context {
            form = form.text("search_context", ctx.to_string());
        }

        let resp = match self
            .http
            .post(format!("{base}/api/chat_stream"))
            .bearer_auth(key)
            .multipart(form)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                sink(AiEvent::Error(format!("Cannot reach Odysseus: {e}")));
                sink(AiEvent::Done);
                return;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            sink(AiEvent::Error(format!("Odysseus answered {status}")));
            sink(AiEvent::Done);
            return;
        }

        let mut stream = resp.bytes_stream();
        let mut buf = Vec::new();

        while let Some(chunk_result) = stream.next().await {
            if cancelled.load(Ordering::Relaxed) {
                break;
            }
            let chunk = match chunk_result {
                Ok(c) => c,
                Err(e) => {
                    sink(AiEvent::Error(format!("Stream error: {e}")));
                    break;
                }
            };
            buf.extend_from_slice(&chunk);

            while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = buf.drain(..=pos).collect();
                let line_str = String::from_utf8_lossy(&line).trim().to_string();
                let Some(data) = line_str.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    break;
                }
                if let Ok(ev) = serde_json::from_str::<Value>(data) {
                    if let Some(t) = ev["delta"].as_str() {
                        sink(AiEvent::Delta(t.to_string()));
                    } else if let Some(sources) = ev["sources"].as_array() {
                        for s in sources {
                            let title = s["title"].as_str().unwrap_or_default().to_string();
                            let url = s["url"].as_str().unwrap_or_default().to_string();
                            if !url.is_empty() {
                                sink(AiEvent::Source { title, url });
                            }
                        }
                    }
                }
            }
        }

        sink(AiEvent::Done);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_root_accepts_both_forms() {
        assert_eq!(local_root(""), "http://127.0.0.1:11434");
        assert_eq!(
            local_root("http://127.0.0.1:11434/"),
            "http://127.0.0.1:11434"
        );
        assert_eq!(
            local_root("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434"
        );
        assert!(model_installed(&["llama3:latest".to_string()], "llama3"));
        assert!(!model_installed(&["mistral:latest".to_string()], "llama3"));
    }

    #[tokio::test]
    async fn disabled_provider_reports_graceful_error() {
        let client = AiClient::new();
        let config = AiConfig::default();
        let cancelled = Arc::new(AtomicBool::new(false));

        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let ev_clone = events.clone();
        client
            .ask(
                &config,
                "What is the speed of light?",
                None,
                cancelled,
                move |ev| {
                    ev_clone.lock().unwrap().push(ev);
                },
            )
            .await;

        let res = events.lock().unwrap().clone();
        assert_eq!(res.len(), 2);
        match &res[0] {
            AiEvent::Error(msg) => assert!(msg.contains("No AI provider configured")),
            _ => panic!("Expected Error event"),
        }
        assert_eq!(res[1], AiEvent::Done);
    }

    #[test]
    fn settings_serialization_never_includes_keys() {
        let config = AiConfig {
            vision_model: String::new(),
            provider: ProviderKind::Cloud,
            endpoint: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            api_key: Some("sk-secret-12345".into()),
        };
        let ui = AiSettingsUi::from(&config);
        assert!(ui.has_key);
        let json_str = serde_json::to_string(&ui).unwrap();
        assert!(!json_str.contains("sk-secret-12345"));
        assert!(!json_str.contains("api_key"));
    }

    #[tokio::test]
    async fn mock_server_stream_openai_compatible() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut req_buf = [0u8; 1024];
            let _ = socket.read(&mut req_buf).await;

            let sse_body = "data: {\"choices\":[{\"delta\":{\"content\":\"Hello \"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"stars!\"}}]}\n\ndata: [DONE]\n\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
                sse_body.len(),
                sse_body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.flush().await.unwrap();
        });

        let client = AiClient::new();
        let config = AiConfig {
            vision_model: String::new(),
            provider: ProviderKind::Local,
            endpoint: format!("http://127.0.0.1:{port}"),
            model: "test-model".into(),
            api_key: None,
        };
        let cancelled = Arc::new(AtomicBool::new(false));

        let events = Arc::new(std::sync::Mutex::new(Vec::new()));
        let ev_clone = events.clone();
        client
            .ask(&config, "Greeting", None, cancelled, move |ev| {
                ev_clone.lock().unwrap().push(ev);
            })
            .await;

        let res = events.lock().unwrap().clone();
        let deltas: Vec<String> = res
            .iter()
            .filter_map(|ev| match ev {
                AiEvent::Delta(d) => Some(d.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(deltas.join(""), "Hello stars!");
        assert_eq!(res.last(), Some(&AiEvent::Done));
    }
}
