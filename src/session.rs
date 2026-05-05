use crate::config::AppConfig;
use crate::launcher;
use crate::ui;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub started_at: String,
    pub working_dir: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claude_session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

fn sessions_path() -> std::path::PathBuf {
    AppConfig::config_dir().join("sessions.json")
}

fn load_sessions() -> Vec<Session> {
    let path = sessions_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    }
}

pub fn load_all() -> Vec<Session> {
    load_sessions()
}

fn save_sessions(sessions: &[Session]) {
    let dir = AppConfig::config_dir();
    fs::create_dir_all(&dir).ok();
    let content = serde_json::to_string_pretty(sessions).expect("failed to serialize sessions");
    fs::write(sessions_path(), content).expect("failed to write sessions");
}

pub fn record(provider: &str, model: &str, claude_session_id: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let session = Session {
        id: id.clone(),
        provider: provider.to_string(),
        model: model.to_string(),
        started_at: Utc::now().to_rfc3339(),
        working_dir: std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        claude_session_id: Some(claude_session_id.to_string()),
        summary: None,
    };
    let mut sessions = load_sessions();
    sessions.push(session);
    if sessions.len() > 100 {
        sessions.drain(..sessions.len() - 100);
    }
    save_sessions(&sessions);
    id
}

pub fn update(id: &str, summary: Option<&str>) {
    let mut sessions = load_sessions();
    if let Some(s) = sessions.iter_mut().find(|s| s.id == id) {
        if let Some(sum) = summary {
            s.summary = Some(sum.to_string());
        }
    }
    save_sessions(&sessions);
}

fn short_dir(path: &str) -> String {
    let p = Path::new(path);
    let mut parts: Vec<&str> = p.iter().filter_map(|c| c.to_str()).collect();
    if parts.len() > 2 {
        parts = parts[parts.len() - 2..].to_vec();
        format!("…/{}", parts.join("/"))
    } else {
        p.display().to_string()
    }
}

fn summary_or_fallback(s: &Session) -> String {
    match &s.summary {
        Some(sum) if !sum.is_empty() => sum.clone(),
        _ => {
            let p = Path::new(&s.working_dir);
            let basename = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&s.working_dir);
            format!("(session in {})", basename)
        }
    }
}

pub fn list() {
    let sessions = load_sessions();
    if sessions.is_empty() {
        ui::warning("no sessions recorded yet");
        return;
    }

    let mut grouped: BTreeMap<String, Vec<&Session>> = BTreeMap::new();
    for s in &sessions {
        let key = format!("[{}] {}", s.provider, s.model);
        grouped.entry(key).or_default().push(s);
    }

    ui::section("Sessions");
    for (group, items) in &grouped {
        println!("  {}", ui::accent(group));
        for s in items.iter().rev().take(10) {
            let ts = &s.started_at[..16];
            let summary = summary_or_fallback(s);
            let resumable = if s.claude_session_id.is_some() {
                ui::accent("+")
            } else {
                ui::muted("-")
            };
            println!(
                "    {} {}  {}  {}  \"{}\"",
                resumable,
                ui::accent(&s.id),
                ui::muted(ts),
                ui::muted(short_dir(&s.working_dir)),
                summary
            );
        }
        println!();
    }
    ui::hint("+ = resumable. Use `ccli resume <id>` or `ccli session resume <id>` to resume.");
    ui::hint("Run `ccli session remove <id>` to delete a session record.");
}

pub fn info(id: &str) {
    let sessions = load_sessions();
    match sessions.iter().find(|s| s.id == id) {
        Some(s) => {
            ui::section("Session details");
            ui::kv("Session", ui::accent(&s.id));
            ui::kv("Provider", ui::accent(&s.provider));
            ui::kv("Model", ui::accent(&s.model));
            ui::kv("Work dir", ui::muted(&s.working_dir));
            ui::kv("Started", ui::muted(&s.started_at));
            ui::kv(
                "Claude SID",
                s.claude_session_id
                    .as_deref()
                    .map(ui::accent)
                    .unwrap_or_else(|| ui::muted("(none)")),
            );
            ui::kv("Summary", s.summary.as_deref().unwrap_or("(none)"));
        }
        None => ui::error(format!("session '{id}' not found")),
    }
}

pub fn resume(id: &str) {
    let sessions = load_sessions();
    let s = sessions.iter().find(|s| s.id == id).unwrap_or_else(|| {
        ui::error(format!("session '{id}' not found"));
        std::process::exit(1);
    });
    let claude_sid = s.claude_session_id.as_deref().unwrap_or_else(|| {
        ui::warning(format!("session '{id}' has no linked Claude session (old format); cannot resume"));
        std::process::exit(1);
    });
    launcher::launch_resume(&s.provider, claude_sid, &s.working_dir);
}

pub fn summarize(id: &str) {
    let sessions = load_sessions();
    let s = sessions.iter().find(|s| s.id == id).unwrap_or_else(|| {
        ui::error(format!("session '{id}' not found"));
        std::process::exit(1);
    });
    let claude_sid = s.claude_session_id.as_deref().unwrap_or_else(|| {
        ui::error(format!("session '{id}' has no linked Claude session; cannot summarize"));
        std::process::exit(1);
    });
    if s.summary.as_deref().unwrap_or("").is_empty() {
        ui::hint("no existing summary — generating via LLM…");
    } else {
        ui::hint(format!("existing summary: \"{}\" — regenerating via LLM…", s.summary.as_deref().unwrap()));
    }
    match launcher::auto_summary(claude_sid, &s.provider) {
        Some(summary) => {
            update(id, Some(&summary));
            ui::success_with_label("summarized", format!("{} → {}", ui::accent(id), ui::accent(&summary)));
        }
        None => {
            ui::warning(format!("could not generate summary for session '{id}' (LLM call may have failed)"));
        }
    }
}

pub fn cat(id: &str) {
    let sessions = load_sessions();
    let s = sessions.iter().find(|s| s.id == id).unwrap_or_else(|| {
        ui::error(format!("session '{id}' not found"));
        std::process::exit(1);
    });
    let claude_sid = s.claude_session_id.as_deref().unwrap_or_else(|| {
        ui::error(format!("session '{id}' has no linked Claude session"));
        std::process::exit(1);
    });

    let claude_home = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude");
    let projects_dir = claude_home.join("projects");

    if !projects_dir.exists() {
        ui::warning("no Claude project data found");
        return;
    }

    for entry in fs::read_dir(&projects_dir).unwrap_or_else(|_| {
        ui::error("failed to read projects dir");
        std::process::exit(1);
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let jsonl = entry.path().join(format!("{claude_sid}.jsonl"));
        if !jsonl.exists() {
            continue;
        }

        let file = fs::File::open(&jsonl).unwrap_or_else(|_| {
            ui::error("failed to open session file");
            std::process::exit(1);
        });
        let reader = BufReader::new(file);

        ui::section(&format!("Session {}", id));
        ui::kv("Provider", ui::accent(&s.provider));
        ui::kv("Model", ui::accent(&s.model));
        println!();

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            let v: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let role = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
            let prefix = match role {
                "human" => format!("{} ", ui::accent("❯")),
                "assistant" => format!("{} ", ui::accent("●")),
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
                println!("{}{}", prefix, t);
                println!();
            }
        }
        return;
    }
    ui::warning(format!("no conversation data found for session '{id}'"));
}

pub fn rename(id: &str, title: &str) {
    if title.trim().is_empty() {
        ui::error("title cannot be empty");
        return;
    }
    let title = title.trim();
    let mut sessions = load_sessions();
    match sessions.iter_mut().find(|s| s.id == id) {
        Some(s) => {
            let old = s.summary.clone().unwrap_or_else(|| "(none)".into());
            s.summary = Some(title.to_string());
            save_sessions(&sessions);
            ui::success_with_label(
                "renamed",
                format!(
                    "{}  \"{}\" {} \"{}\"",
                    ui::accent(id),
                    ui::muted(&old),
                    ui::muted("→"),
                    ui::accent(title),
                ),
            );
        }
        None => ui::error(format!("session '{id}' not found")),
    }
}

pub fn remove(id: &str) {
    let sessions = load_sessions();
    let s = match sessions.iter().find(|s| s.id == id) {
        Some(s) => s,
        None => {
            ui::error(format!("session '{id}' not found"));
            return;
        }
    };

    // Show what will be removed
    let summary = s.summary.as_deref().unwrap_or("(no summary)");
    ui::section("Removing session");
    ui::kv("ID", ui::accent(&s.id));
    ui::kv("Provider", ui::muted(&s.provider));
    ui::kv("Model", ui::muted(&s.model));
    ui::kv("Title", summary);

    let remaining: Vec<_> = sessions.into_iter().filter(|s| s.id != id).collect();
    save_sessions(&remaining);
    ui::success_with_label("removed", format!("session {}", ui::accent(id)));
}
