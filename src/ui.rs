use owo_colors::{OwoColorize, Style};
use std::env;
use std::fmt::Display;
use std::io::{self, IsTerminal};

fn color_enabled_for(stream_is_terminal: bool) -> bool {
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if env::var("CLICOLOR_FORCE").ok().as_deref() == Some("1") {
        return true;
    }
    if env::var("CLICOLOR").ok().as_deref() == Some("0") {
        return false;
    }
    stream_is_terminal
}

fn stdout_color() -> bool {
    color_enabled_for(io::stdout().is_terminal())
}

fn stderr_color() -> bool {
    color_enabled_for(io::stderr().is_terminal())
}

fn accent_style(enabled: bool) -> Style {
    if enabled {
        Style::new().cyan().bold()
    } else {
        Style::new()
    }
}

fn muted_style(enabled: bool) -> Style {
    if enabled {
        Style::new().dimmed()
    } else {
        Style::new()
    }
}

fn success_style(enabled: bool) -> Style {
    if enabled {
        Style::new().green().bold()
    } else {
        Style::new()
    }
}

fn warning_style(enabled: bool) -> Style {
    if enabled {
        Style::new().yellow().bold()
    } else {
        Style::new()
    }
}

fn error_style(enabled: bool) -> Style {
    if enabled {
        Style::new().red().bold()
    } else {
        Style::new()
    }
}

fn section_style(enabled: bool) -> Style {
    if enabled {
        Style::new().blue().bold()
    } else {
        Style::new()
    }
}

pub fn accent(text: impl Display) -> String {
    let enabled = stdout_color();
    format!("{}", text.style(accent_style(enabled)))
}

pub fn muted(text: impl Display) -> String {
    let enabled = stdout_color();
    format!("{}", text.style(muted_style(enabled)))
}

pub fn prompt_label(label: &str) -> String {
    let enabled = stdout_color();
    format!("{}", label.style(accent_style(enabled)))
}

pub fn section(title: &str) {
    let enabled = stdout_color();
    println!("\n{}", title.style(section_style(enabled)));
}

pub fn success_with_label(label: &str, msg: impl Display) {
    let enabled = stdout_color();
    println!("{} {}", label.style(success_style(enabled)), msg);
}

pub fn warning(msg: impl Display) {
    let enabled = stderr_color();
    eprintln!("{} {}", "warning".style(warning_style(enabled)), msg);
}

pub fn error(msg: impl Display) {
    let enabled = stderr_color();
    eprintln!("{} {}", "error".style(error_style(enabled)), msg);
}

pub fn kv(label: &str, value: impl Display) {
    let enabled = stdout_color();
    println!("  {:<12} {}", label.style(muted_style(enabled)), value);
}

pub fn hint(msg: impl Display) {
    let enabled = stdout_color();
    println!("{}", msg.style(muted_style(enabled)));
}
