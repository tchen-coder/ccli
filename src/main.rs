mod config;
mod launcher;
mod provider;
mod session;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ccli", version, about = "Compatible LLM CLI - switch providers for Claude Code")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch Claude Code with a specific provider
    Use {
        /// Provider profile name
        name: String,
    },
    /// Manage LLM providers
    Llm {
        #[command(subcommand)]
        action: LlmAction,
    },
    /// View session history
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },
    /// Show current config path and default provider
    Config,
}

#[derive(Subcommand)]
enum LlmAction {
    /// Add a new provider profile
    Add,
    /// List all provider profiles
    List,
    /// Remove a provider profile
    Remove { name: String },
    /// Set the default provider
    SetDefault { name: String },
}

#[derive(Subcommand)]
enum SessionAction {
    /// List recent sessions
    List,
    /// Show session details
    Info { id: String },
    /// Resume a previous session
    Resume { id: String },
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
