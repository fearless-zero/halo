use crate::types::NotesToken;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

/// A running local llama.cpp server dedicated to note generation.
pub struct LlmServer {
    child: Option<CommandChild>,
    port: u16,
}

fn free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

async fn wait_ready(port: u16) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/health");
    for _ in 0..240 {
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Err(anyhow!("llama-server did not become ready in time"))
}

impl LlmServer {
    pub async fn start(app: &AppHandle, model_path: &Path) -> Result<LlmServer> {
        if !model_path.exists() {
            return Err(anyhow!("Language model is not installed"));
        }
        let port = free_port()?;
        let (mut rx, child) = app
            .shell()
            .sidecar("llama-server")
            .context("llama-server sidecar not found — run scripts/fetch-sidecars.sh")?
            .args([
                "-m",
                &model_path.to_string_lossy(),
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "-c",
                "8192",
                "-ngl",
                "99",
            ])
            .spawn()
            .context("failed to start llama-server")?;

        // Drain server output so its stdout/stderr pipe never fills and blocks.
        tauri::async_runtime::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let CommandEvent::Stderr(bytes) = event {
                    let line = String::from_utf8_lossy(&bytes);
                    if line.contains("error") {
                        eprintln!("llama-server: {}", line.trim_end());
                    }
                }
            }
        });

        wait_ready(port).await?;
        Ok(LlmServer { child: Some(child), port })
    }

    pub fn stop(&mut self) {
        if let Some(child) = self.child.take() {
            let _ = child.kill();
        }
    }

    /// Stream a completion, emitting `notes-token` events as text arrives, and
    /// return the full generated text.
    pub async fn generate(&self, app: &AppHandle, note_id: &str, prompt: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.port);
        let body = json!({
            "model": "local",
            "stream": true,
            "temperature": 0.3,
            "messages": [{ "role": "user", "content": prompt }],
            "cache_prompt": true
        });

        let resp = client.post(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("note generation failed: HTTP {}", resp.status()));
        }

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut output = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer.drain(..=pos);
                let Some(data) = line.strip_prefix("data:") else { continue };
                let data = data.trim();
                if data == "[DONE]" {
                    continue;
                }
                if let Ok(value) = serde_json::from_str::<Value>(data) {
                    if let Some(token) = value["choices"][0]["delta"]["content"].as_str() {
                        if !token.is_empty() {
                            output.push_str(token);
                            let _ = app.emit(
                                "notes-token",
                                NotesToken { note_id: note_id.to_string(), text: token.to_string() },
                            );
                        }
                    }
                }
            }
        }

        let _ = app.emit("notes-done", note_id.to_string());
        Ok(output.trim().to_string())
    }
}

impl Drop for LlmServer {
    fn drop(&mut self) {
        self.stop();
    }
}
