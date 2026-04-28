mod config;
mod launcher;
mod provider;
mod session;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "ccli",
    version,
    about = "Switch LLM providers for Claude Code CLI",
    long_about = "ccli is a lightweight wrapper that launches Claude Code with any Anthropic-compatible\n\
                  API provider. It manages provider profiles, handles authentication, and tracks\n\
                  session history with resume support.\n\n\
                  Quick start:\n  \
                  ccli llm add          Add a provider (interactive)\n  \
                  ccli use <provider>   Launch Claude Code with that provider\n  \
                  ccli                  Launch with default provider"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch Claude Code with a specific provider
    Use {
        /// Provider profile name (as shown in `ccli llm list`)
        name: String,
    },
    /// Manage LLM provider profiles
    Llm {
        #[command(subcommand)]
        action: LlmAction,
    },
    /// View and manage session history
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Show config file path and current default provider
    Config,
}

#[derive(Subcommand)]
enum LlmAction {
    /// Add a new provider profile (interactive, with presets)
    Add,
    /// List all configured provider profiles
    List,
    /// Remove a provider profile by name
    Remove {
        /// Provider profile name to remove
        name: String,
    },
    /// Set the default provider used when running `ccli` without arguments
    SetDefault {
        /// Provider profile name to set as default
        name: String,
    },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List recent sessions grouped by provider and model
    List,
    /// Show detailed info for a specific session
    Info {
        /// Session ID (short hash shown in `ccli session list`)
        id: String,
    },
    /// Resume a previous Claude Code session
    Resume {
        /// Session ID to resume (must have a linked Claude session)
        id: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Use { name }) => launcher::launch(Some(&name)),
        Some(Commands::Llm { action }) => match action {
            LlmAction::Add => provider::add(),
            LlmAction::List => provider::list(),
            LlmAction::Remove { name } => provider::remove(&name),
            LlmAction::SetDefault { name } => provider::set_default(&name),
        },
        Some(Commands::Session { action }) => match action {
            SessionAction::List => session::list(),
            SessionAction::Info { id } => session::info(&id),
            SessionAction::Resume { id } => session::resume(&id),
        },
        Some(Commands::Config) => {
            let config = config::AppConfig::load();
            println!("Config: {}", config::AppConfig::config_path().display());
            println!("Default: {}", config.default_provider.as_deref().unwrap_or("(none)"));
        }
        None => launcher::launch(None),
    }
}
