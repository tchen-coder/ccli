use crate::config::{AppConfig, Provider};
use std::io::{self, Write};

fn prompt(label: &str) -> String {
    print!("{label}: ");
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
        print!("{label}: ");
    } else {
        print!("{label} [{default}]: ");
    }
    io::stdout().flush().unwrap();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    let val = buf.trim().to_string();
    if val.is_empty() { default.to_string() } else { val }
}

pub fn add() {
    println!("Available presets:");
    for (i, p) in PRESETS.iter().enumerate() {
        println!("  {}. {} - {} (model: {})", i + 1, p.key, p.base_url, p.model);
    }
    println!("  0. Custom provider");
    println!();

    let choice = prompt("Select (0-4)");
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

    println!("Enter your API key, or type env:VAR_NAME to read from an environment variable.");
    println!("  Example: sk-abc123... or env:DEEPSEEK_API_KEY");
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
    println!("Provider '{key}' added. Run `ccli use {key}` to start Claude Code with it.");
}

pub fn remove(name: &str) {
    let mut config = AppConfig::load();
    if config.providers.remove(name).is_some() {
        if config.default_provider.as_deref() == Some(name) {
            config.default_provider = config.providers.keys().next().cloned();
        }
        config.save().expect("failed to save config");
        println!("Provider '{name}' removed.");
    } else {
        eprintln!("Provider '{name}' not found.");
    }
}

pub fn set_default(name: &str) {
    let mut config = AppConfig::load();
    if config.providers.contains_key(name) {
        config.default_provider = Some(name.to_string());
        config.save().expect("failed to save config");
        println!("Default provider set to '{name}'.");
    } else {
        eprintln!("Provider '{name}' not found.");
    }
}

pub fn list() {
    let config = AppConfig::load();
    if config.providers.is_empty() {
        println!("No providers configured. Run `ccli provider add` to add one.");
        return;
    }
    let default = config.default_provider.as_deref().unwrap_or("");
    for (key, p) in &config.providers {
        let marker = if key == default { " *" } else { "" };
        println!("  {key}{marker}  ({})  model={}", p.base_url, p.model);
    }
}
