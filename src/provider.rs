use crate::config::{AppConfig, Provider};
use crate::ui;
use std::io::{self, Write};

fn prompt(label: &str) -> String {
    print!("{}: ", ui::prompt_label(label));
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    buf.trim().to_string()
}

struct Preset {
    key: &'static str,
    name: &'static str,
    base_url: &'static str,
    model: &'static str,
}

const PRESETS: &[Preset] = &[
    Preset { key: "anthropic", name: "Anthropic Claude", base_url: "https://api.anthropic.com", model: "claude-sonnet-4-6" },
    Preset { key: "deepseek", name: "DeepSeek", base_url: "https://api.deepseek.com/anthropic", model: "deepseek-v4-pro" },
    Preset { key: "openrouter", name: "OpenRouter", base_url: "https://openrouter.ai/api/v1", model: "anthropic/claude-opus-4-7" },
    Preset { key: "mimo", name: "MiMo (Xiaomi)", base_url: "https://api.xiaomimimo.com/anthropic", model: "mimo-v2-flash" },
];

fn prompt_with_default(label: &str, default: &str) -> String {
    if default.is_empty() {
        print!("{}: ", ui::prompt_label(label));
    } else {
        print!("{} [{}]: ", ui::prompt_label(label), ui::muted(default));
    }
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    let val = buf.trim().to_string();
    if val.is_empty() { default.to_string() } else { val }
}

pub fn add() {
    ui::section("Provider presets");
    for (i, p) in PRESETS.iter().enumerate() {
        println!(
            "  {} {}  {}  {}",
            ui::accent(format!("{}.", i + 1)),
            ui::accent(p.key),
            ui::muted(p.base_url),
            ui::muted(format!("model={}", p.model))
        );
    }
    println!("  {} {}", ui::accent("0."), ui::muted("Custom provider"));
    println!();

    let choice = prompt("Select");
    let preset = choice.parse::<usize>().ok().and_then(|i| {
        if i == 0 { None } else { PRESETS.get(i - 1) }
    });

    let (key, name, base_url, model);

    if let Some(p) = preset {
        key = p.key.to_string();
        name = p.name.to_string();
        base_url = prompt_with_default("API base URL", p.base_url);
        model = prompt_with_default("Model", p.model);
        println!();
    } else {
        key = prompt("Provider key (used in `ccli use <key>`, e.g. my-llm)");
        name = prompt("Provider name (e.g. My LLM Service)");
        base_url = prompt("API base URL (e.g. https://api.example.com/v1)");
        model = prompt("Model (e.g. gpt-4o)");
        println!();
    }

    ui::hint("Enter your API key, or type env:VAR_NAME to read from an environment variable.");
    ui::hint("Example: sk-abc123... or env:DEEPSEEK_API_KEY");
    let api_input = prompt("API key");

    let (api_key, api_key_env) = if let Some(var) = api_input.strip_prefix("env:") {
        (None, Some(var.to_string()))
    } else {
        (Some(api_input), None)
    };

    let provider = Provider { name, base_url, api_key, api_key_env, model };
    let mut config = AppConfig::load();
    config.providers.insert(key.clone(), provider);
    if config.default_provider.is_none() {
        config.default_provider = Some(key.clone());
    }
    config.save().expect("failed to save config");
    println!();
    ui::success_with_label("added", format!("provider {}", ui::accent(format!("'{key}'"))));
    ui::hint(format!("Run `ccli use {key}` to start Claude Code."));
}

pub fn remove(name: &str) {
    let mut config = AppConfig::load();
    if config.providers.remove(name).is_some() {
        if config.default_provider.as_deref() == Some(name) {
            config.default_provider = config.providers.keys().next().cloned();
        }
        config.save().expect("failed to save config");
        ui::success_with_label("removed", format!("provider {}", ui::accent(format!("'{name}'"))));
    } else {
        ui::error(format!("provider '{name}' not found"));
    }
}

pub fn set_default(name: &str) {
    let mut config = AppConfig::load();
    if config.providers.contains_key(name) {
        config.default_provider = Some(name.to_string());
        config.save().expect("failed to save config");
        ui::success_with_label("default", format!("provider set to {}", ui::accent(format!("'{name}'"))));
    } else {
        ui::error(format!("provider '{name}' not found"));
    }
}

pub fn list() {
    let config = AppConfig::load();
    if config.providers.is_empty() {
        ui::warning("no providers configured; run `ccli llm add`");
        return;
    }
    let default = config.default_provider.as_deref().unwrap_or("");
    ui::section("Providers");
    for (key, p) in &config.providers {
        let marker = if key == default {
            format!(" {}", ui::accent("(default)"))
        } else {
            String::new()
        };
        println!(
            "  {}{}  {}  {}",
            ui::accent(key),
            marker,
            ui::muted(format!("model={}", p.model)),
            ui::muted(format!("({})", p.base_url))
        );
    }
}
