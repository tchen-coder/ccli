use crate::config::AppConfig;
use crate::session;
use crate::ui;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

fn write_launch_settings(base_url: &str, api_key: &str, claude_session_id: &str) -> String {
    let settings_dir = AppConfig::config_dir().join("launch");
    fs::create_dir_all(&settings_dir).expect("failed to create launch dir");
    // Per-session file to avoid conflicts between concurrent windows
    let path = settings_dir.join(format!("{}.json", &claude_session_id[..8]));
    let content = format!(
        r#"{{"env":{{"ANTHROPIC_BASE_URL":"{}","ANTHROPIC_AUTH_TOKEN":"{}"}}}}"#,
        base_url, api_key
    );
    fs::write(&path, &content).expect("failed to write launch settings");
    path.display().to_string()
}

fn claude_version() -> String {
    Command::new("claude")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| "unknown".into())
        .trim()
        .to_string()
}

fn sanitize_claude_env(command: &mut Command) -> &mut Command {
    command
        .env_remove("ANTHROPIC_AUTH_TOKEN")
        .env_remove("ANTHROPIC_BASE_URL")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("ANTHROPIC_BEDROCK_BASE_URL")
        .env_remove("ANTHROPIC_VERTEX_BASE_URL")
        .env_remove("ANTHROPIC_FOUNDRY_BASE_URL")
        .env_remove("CLAUDE_CODE_USE_BEDROCK")
        .env_remove("CLAUDE_CODE_USE_VERTEX")
}

fn resolve_provider(provider_key: Option<&str>) -> (String, String, String, String) {
    let config = AppConfig::load();
    let key = provider_key
        .map(String::from)
        .or(config.default_provider.clone())
        .unwrap_or_else(|| {
            ui::error("no provider specified and no default set; run `ccli llm add` first");
            std::process::exit(1);
        });
    let provider = config.providers.get(&key).unwrap_or_else(|| {
        ui::error(format!("provider '{key}' not found; run `ccli llm list`"));
        std::process::exit(1);
    });
    let api_key = provider
        .api_key.clone()
        .or_else(|| provider.api_key_env.as_ref().and_then(|v| env::var(v).ok()))
        .unwrap_or_else(|| {
            ui::error(format!("no API key for provider '{key}'"));
            std::process::exit(1);
        });
    (key, provider.base_url.clone(), api_key, provider.model.clone())
}

fn build_claude_command(
    settings_path: &str,
    model: &str,
    claude_session_id: &str,
) -> Command {
    let mut cmd = Command::new("claude");
    sanitize_claude_env(&mut cmd);
    cmd.arg("--bare")
        .arg("--settings")
        .arg(settings_path)
        .arg("--model")
        .arg(model)
        .arg("--session-id")
        .arg(claude_session_id);
    cmd
}

fn normalize_summary(raw: &str) -> Option<String> {
    let text = raw
        .replace(['\n', '\r', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let text = text.trim().to_string();
    if text.is_empty() || text.len() < 4 {
        return None;
    }
    // Filter Claude Code slash commands — not meaningful as titles
    let cc_slash = ["/plan", "/init", "/compact", "/clear", "/help", "/doctor",
                    "/bug", "/cost", "/context", "/review", "/ide", "/pr"];
    let lower = text.to_lowercase();
    if cc_slash.iter().any(|cmd| lower.starts_with(cmd)) {
        return None;
    }
    let truncated = if text.chars().count() > 72 {
        let s: String = text.chars().take(69).collect();
        format!("{s}...")
    } else {
        text
    };
    Some(truncated)
}

fn claude_home() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
}

fn extract_summary_from_history(claude_session_id: &str) -> Option<String> {
    let path = claude_home().join("history.jsonl");
    let file = fs::File::open(&path).ok()?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.ok()?;
        if !line.contains(claude_session_id) {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(&line).ok()?;
        if v.get("sessionId").and_then(|s| s.as_str()) != Some(claude_session_id) {
            continue;
        }
        if let Some(display) = v.get("display").and_then(|d| d.as_str()) {
            if let Some(s) = normalize_summary(display) {
                return Some(s);
            }
        }
    }
    None
}

fn extract_summary_from_project(claude_session_id: &str) -> Option<String> {
    let projects_dir = claude_home().join("projects");
    if !projects_dir.exists() {
        return None;
    }
    for entry in fs::read_dir(&projects_dir).ok()? {
        let entry = entry.ok()?;
        let jsonl = entry.path().join(format!("{claude_session_id}.jsonl"));
        if !jsonl.exists() {
            continue;
        }
        let file = fs::File::open(&jsonl).ok()?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.ok()?;
            let v: serde_json::Value = serde_json::from_str(&line).ok()?;
            if v.get("type").and_then(|t| t.as_str()) != Some("human") {
                continue;
            }
            let text = v.get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| {
                    if let Some(s) = c.as_str() {
                        return Some(s.to_string());
                    }
                    c.as_array().and_then(|arr| {
                        arr.iter().find_map(|item| {
                            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                item.get("text").and_then(|t| t.as_str()).map(String::from)
                            } else {
                                None
                            }
                        })
                    })
                });
            if let Some(t) = text {
                if let Some(s) = normalize_summary(&t) {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn collect_conversation_for_summary(claude_session_id: &str) -> Option<String> {
    let projects_dir = claude_home().join("projects");
    if !projects_dir.exists() {
        return None;
    }
    for entry in fs::read_dir(&projects_dir).ok()? {
        let entry = entry.ok()?;
        let jsonl = entry.path().join(format!("{claude_session_id}.jsonl"));
        if !jsonl.exists() {
            continue;
        }
        let file = fs::File::open(&jsonl).ok()?;
        let reader = BufReader::new(file);
        let mut parts: Vec<String> = Vec::new();
        let mut total_chars = 0usize;
        let max_chars = 1200;

        for line in reader.lines() {
            let line = line.ok()?;
            let v: serde_json::Value = serde_json::from_str(&line).ok()?;
            let msg_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let role = match msg_type {
                "human" => "User",
                "assistant" => "Assistant",
                _ => continue,
            };

            let text = v.get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| {
                    if let Some(s) = c.as_str() {
                        return Some(s.to_string());
                    }
                    c.as_array().and_then(|arr| {
                        arr.iter().find_map(|item| {
                            if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                                item.get("text").and_then(|t| t.as_str()).map(String::from)
                            } else {
                                None
                            }
                        })
                    })
                });

            if let Some(t) = text {
                let clean = t.replace(['\n', '\r', '\t'], " ").trim().to_string();
                if clean.len() < 4 {
                    continue;
                }
                // Truncate each message to keep total within bounds
                let max_part = 300usize.saturating_sub(parts.len() * 10);
                let truncated = if clean.chars().count() > max_part {
                    let s: String = clean.chars().take(max_part.saturating_sub(3)).collect();
                    format!("{s}...")
                } else {
                    clean
                };
                if total_chars + truncated.len() > max_chars {
                    break;
                }
                total_chars += truncated.len();
                parts.push(format!("{role}: {truncated}"));
            }
        }

        if parts.is_empty() {
            return None;
        }
        return Some(parts.join("\n"));
    }
    None
}

fn generate_summary_via_llm(
    conversation: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Option<String> {
    let endpoint = format!("{}/v1/messages", base_url.trim_end_matches('/'));

    let body = ureq::json!({
        "model": model,
        "max_tokens": 80,
        "temperature": 0.0,
        "system": "You are a session title generator. Analyze the conversation and output ONLY a concise title (2-6 words, under 40 characters) that captures the user's main task or question. Use the same language as the user. Be specific: prefer 'Fix Rust borrow checker error in auth module' over 'Rust help'. No quotes, no prefixes, no explanations, no punctuation at the end.",
        "messages": [
            {
                "role": "user",
                "content": format!("Generate a title for this conversation:\n\n{conversation}")
            }
        ]
    });

    let resp = ureq::post(&endpoint)
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .set("User-Agent", "claude-code/1.0.0 (ccli)")
        .timeout(Duration::from_secs(15))
        .send_json(body)
        .ok()?;

    let json: serde_json::Value = resp.into_json().ok()?;
    let title = json
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|arr| arr.first())
        .and_then(|block| block.get("text"))
        .and_then(|t| t.as_str())
        .map(|s| s.trim().trim_matches('"').trim().to_string());

    match title {
        Some(t) if t.len() >= 4 && t.len() <= 100 => Some(t),
        _ => None,
    }
}

pub fn auto_summary(claude_session_id: &str, provider_key: &str) -> Option<String> {
    // Priority 1: LLM-generated summary (best quality, content-aware)
    if let Some(conversation) = collect_conversation_for_summary(claude_session_id) {
        let config = AppConfig::load();
        if let Some(provider) = config.providers.get(provider_key) {
            let api_key = provider
                .api_key
                .clone()
                .or_else(|| {
                    provider
                        .api_key_env
                        .as_ref()
                        .and_then(|v| env::var(v).ok())
                });
            if let Some(key) = api_key {
                ui::hint("generating session summary via LLM…");
                if let Some(summary) = generate_summary_via_llm(
                    &conversation,
                    &provider.base_url,
                    &key,
                    &provider.model,
                ) {
                    return Some(summary);
                }
            }
        }
    }

    // Priority 2: display field from history.jsonl
    if let Some(s) = extract_summary_from_history(claude_session_id) {
        return Some(s);
    }

    // Priority 3: first human message from project jsonl
    extract_summary_from_project(claude_session_id)
}

pub fn launch(provider_key: Option<&str>) {
    let (key, base_url, api_key, model) = resolve_provider(provider_key);
    let claude_sid = uuid::Uuid::new_v4().to_string();
    let settings_path = write_launch_settings(&base_url, &api_key, &claude_sid);
    let ccli_id = session::record(&key, &model, &claude_sid);

    ui::section("Launching Claude Code");
    ui::kv("Claude", ui::muted(claude_version()));
    ui::kv("Provider", ui::accent(&key));
    ui::kv("Model", ui::accent(&model));
    ui::kv("Endpoint", ui::muted(&base_url));
    ui::kv("Session", format!("{}  {}", ui::accent(&ccli_id), ui::muted(format!("→ claude:{}", &claude_sid[..8]))));

    let mut cmd = build_claude_command(&settings_path, &model, &claude_sid);
    let status = cmd.status();

    match status {
        Ok(s) if s.success() || s.code().is_some() => {}
        Ok(s) => ui::warning(format!("Claude exited with: {s}")),
        Err(e) => {
            ui::error(format!("failed to launch claude: {e}"));
            std::process::exit(1);
        }
    }

    let summary = auto_summary(&claude_sid, &key);
    if let Some(ref s) = summary {
        ui::kv("Summary", ui::accent(s));
    }
    if summary.is_some() {
        session::update(&ccli_id, summary.as_deref());
    }
    ui::hint(format!("Resume with: ccli resume {}", ui::accent(&ccli_id)));
}

pub fn launch_resume(provider_key: &str, claude_session_id: &str, working_dir: &str) {
    let (_, base_url, api_key, model) = resolve_provider(Some(provider_key));
    let settings_path = write_launch_settings(&base_url, &api_key, claude_session_id);

    ui::section("Resuming Claude Code session");
    ui::kv("Claude", ui::muted(claude_version()));
    ui::kv("Provider", ui::accent(provider_key));
    ui::kv("Model", ui::accent(&model));
    ui::kv("Endpoint", ui::muted(&base_url));
    ui::kv("Session", ui::accent(&claude_session_id[..8]));

    let mut cmd = Command::new("claude");
    sanitize_claude_env(&mut cmd);
    cmd.arg("--bare")
        .arg("--settings")
        .arg(&settings_path)
        .arg("--model")
        .arg(&model)
        .arg("--resume")
        .arg(claude_session_id);

    let wd = PathBuf::from(working_dir);
    if wd.exists() {
        cmd.current_dir(&wd);
    }

    match cmd.status() {
        Ok(_) => {}
        Err(e) => {
            ui::error(format!("failed to launch claude: {e}"));
            std::process::exit(1);
        }
    }

    // Refresh summary after resumed session ends (only if empty)
    let sessions = crate::session::load_all();
    let needs_summary = sessions
        .iter()
        .find(|s| s.claude_session_id.as_deref() == Some(claude_session_id))
        .map(|s| s.summary.as_deref().unwrap_or("").is_empty())
        .unwrap_or(true);
    if needs_summary {
        if let Some(summary) = auto_summary(claude_session_id, provider_key) {
            ui::kv("Summary", ui::accent(&summary));
            if let Some(s) = sessions.iter().find(|s| s.claude_session_id.as_deref() == Some(claude_session_id)) {
                session::update(&s.id, Some(&summary));
                ui::hint(format!("Resume with: ccli resume {}", ui::accent(&s.id)));
            }
        }
    }
}
