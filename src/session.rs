use crate::config::AppConfig;
use crate::launcher;
use crate::ui;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

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
            let summary = s.summary.as_deref().unwrap_or("-");
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
                ui::muted(&s.working_dir),
                summary
            );
        }
        println!();
    }
    ui::hint("+ = resumable. Use `ccli session resume <id>` to resume.");
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
