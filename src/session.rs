use crate::config::AppConfig;
use crate::launcher;
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
        println!("No sessions recorded yet.");
        return;
    }

    let mut grouped: BTreeMap<String, Vec<&Session>> = BTreeMap::new();
    for s in &sessions {
        let key = format!("[{}] {}", s.provider, s.model);
        grouped.entry(key).or_default().push(s);
    }

    for (group, items) in &grouped {
        println!("\n  {group}");
        for s in items.iter().rev().take(10) {
            let ts = &s.started_at[..16];
            let summary = s.summary.as_deref().unwrap_or("-");
            let resumable = if s.claude_session_id.is_some() { "+" } else { " " };
            println!("    {}{} {}  {}  \"{}\"", resumable, s.id, ts, s.working_dir, summary);
        }
    }
    println!();
    println!("  + = resumable. Use `ccli session resume <id>` to resume.");
}

pub fn info(id: &str) {
    let sessions = load_sessions();
    match sessions.iter().find(|s| s.id == id) {
        Some(s) => {
            println!("Session:    {}", s.id);
            println!("  Provider:    {}", s.provider);
            println!("  Model:       {}", s.model);
            println!("  Working dir: {}", s.working_dir);
            println!("  Started at:  {}", s.started_at);
            println!("  Claude SID:  {}", s.claude_session_id.as_deref().unwrap_or("(none)"));
            println!("  Summary:     {}", s.summary.as_deref().unwrap_or("(none)"));
        }
        None => eprintln!("Session '{id}' not found."),
    }
}

pub fn resume(id: &str) {
    let sessions = load_sessions();
    let s = sessions.iter().find(|s| s.id == id).unwrap_or_else(|| {
        eprintln!("Session '{id}' not found.");
        std::process::exit(1);
    });
    let claude_sid = s.claude_session_id.as_deref().unwrap_or_else(|| {
        eprintln!("Session '{id}' has no linked Claude session (old format). Cannot resume.");
        std::process::exit(1);
    });
    launcher::launch_resume(&s.provider, claude_sid, &s.working_dir);
}
