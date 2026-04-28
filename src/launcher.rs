use crate::config::AppConfig;
use crate::session;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Command;

fn write_launch_settings(base_url: &str, api_key: &str) -> String {
    let settings_dir = AppConfig::config_dir().join("launch");
    fs::create_dir_all(&settings_dir).expect("failed to create launch dir");
    let path = settings_dir.join("settings.json");
    let content = format!(
        r#"{{"env":{{"ANTHROPIC_BASE_URL":"{}","ANTHROPIC_AUTH_TOKEN":"{}"}}}}"#,
        base_url, api_key
    );
    fs::write(&path, &content).expect("failed to write launch settings");
    path.display().to_string()
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
            eprintln!("No provider specified and no default set. Run `ccli llm add` first.");
            std::process::exit(1);
        });
    let provider = config.providers.get(&key).unwrap_or_else(|| {
        eprintln!("Provider '{key}' not found. Run `ccli llm list`.");
        std::process::exit(1);
    });
    let api_key = provider
        .api_key.clone()
        .or_else(|| provider.api_key_env.as_ref().and_then(|v| env::var(v).ok()))
        .unwrap_or_else(|| {
            eprintln!("No API key for provider '{key}'.");
            std::process::exit(1);
        });
    (key, provider.base_url.clone(), api_key, provider.model.clone())
}

fn build_claude_command(
    settings_path: &str,
    model: &str,
    claude_session_id: &str,
    ccli_id: &str,
) -> Command {
    let mut cmd = Command::new("claude");
    sanitize_claude_env(&mut cmd);
    cmd.arg("--bare")
        .arg("--settings")
        .arg(settings_path)
        .arg("--model")
        .arg(model)
        .arg("--session-id")
        .arg(claude_session_id)
        .arg("--name")
        .arg(format!("ccli:{ccli_id}"));
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
    let skip_prefixes = ["/", "git ", "cargo ", "curl ", "claude ", "ccli "];
    let lower = text.to_lowercase();
    if skip_prefixes.iter().any(|p| lower.starts_with(p)) {
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

pub fn launch(provider_key: Option<&str>) {
    let (key, base_url, api_key, model) = resolve_provider(provider_key);
    let settings_path = write_launch_settings(&base_url, &api_key);
    let claude_sid = uuid::Uuid::new_v4().to_string();
    let ccli_id = session::record(&key, &model, &claude_sid);

    println!("Launching Claude Code with [{key}] model={model}");
    println!("  session: {ccli_id} → claude:{}", &claude_sid[..8]);

    let mut cmd = build_claude_command(&settings_path, &model, &claude_sid, &ccli_id);
    let status = cmd.status();

    match status {
        Ok(s) if s.success() || s.code().is_some() => {}
        Ok(s) => eprintln!("Claude exited with: {s}"),
        Err(e) => {
            eprintln!("Failed to launch claude: {e}");
            std::process::exit(1);
        }
    }

    let summary = extract_summary_from_history(&claude_sid)
        .or_else(|| extract_summary_from_project(&claude_sid));
    if summary.is_some() {
        session::update(&ccli_id, summary.as_deref());
    }
}

pub fn launch_resume(provider_key: &str, claude_session_id: &str, working_dir: &str) {
    let (_, base_url, api_key, model) = resolve_provider(Some(provider_key));
    let settings_path = write_launch_settings(&base_url, &api_key);

    println!("Resuming Claude session {}", &claude_session_id[..8]);

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
            eprintln!("Failed to launch claude: {e}");
            std::process::exit(1);
        }
    }
}
