# ccli

Switch LLM providers for Claude Code CLI.

`ccli` is a lightweight wrapper that launches [Claude Code](https://docs.anthropic.com/en/docs/claude-code) with any Anthropic-compatible API provider — no proxy, no translation layer.

## How It Works

```
ccli use deepseek
    │
    ├─ Reads provider config from ~/.ccli/config.toml
    ├─ Writes a temporary settings.json with ANTHROPIC_BASE_URL + ANTHROPIC_AUTH_TOKEN
    ├─ Generates a session UUID for deterministic session binding
    └─ Spawns: claude --bare --settings <path> --model <model> --session-id <uuid>
```

Only providers that expose an **Anthropic-compatible Messages API** endpoint are supported. No OpenAI-to-Anthropic translation is performed.

## Supported Providers

| Provider | Endpoint | Notes |
|----------|----------|-------|
| Anthropic | `https://api.anthropic.com` | Native |
| DeepSeek | `https://api.deepseek.com/anthropic` | Anthropic-compatible |
| MiMo (Xiaomi) | `https://api.xiaomimimo.com/anthropic` | Anthropic-compatible |
| OpenRouter | `https://openrouter.ai/api/v1` | Multi-model gateway |

Custom providers can be added if they implement the Anthropic Messages API.

## Install

```bash
git clone <repo-url> && cd compatible_llm_cli
cargo install --path .
```

## Cross-Platform Build

```bash
# macOS (default)
cargo build --release

# Linux x86_64
cargo build --release --target x86_64-unknown-linux-gnu

# Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu
```

Binary output: `target/release/ccli` (or `target/<target>/release/ccli`).

## Usage

```bash
# Add a provider (interactive, with presets)
ccli llm add

# List configured providers
ccli llm list

# Set default provider
ccli llm set-default deepseek

# Launch Claude Code with a provider
ccli use deepseek

# Launch with default provider
ccli

# View session history
ccli session list

# Resume a previous session
ccli session resume <id>
```

## Example

```bash
$ ccli llm add
Available presets:
  1. anthropic - https://api.anthropic.com (model: claude-sonnet-4-6)
  2. deepseek - https://api.deepseek.com/anthropic (model: deepseek-v4-pro)
  3. openrouter - https://openrouter.ai/api/v1 (model: anthropic/claude-opus-4-7)
  4. mimo - https://api.xiaomimimo.com/anthropic (model: mimo-v2-flash)
  0. Custom provider

Select (0-4): 2
API base URL [https://api.deepseek.com/anthropic]:
Model [deepseek-v4-pro]:
API key: sk-xxx...

Provider 'deepseek' added.

$ ccli use deepseek
Launching Claude Code with [deepseek] model=deepseek-v4-pro
  session: a1b2c3d4 → claude:5e6f7a8b
# ... Claude Code starts ...

$ ccli session list
  [deepseek] deepseek-v4-pro
    +a1b2c3d4 2026-04-29T10:30  /Users/me/project  "Implement auth middleware"
```

## Config

Config file: `~/.ccli/config.toml`

```toml
default_provider = "deepseek"

[providers.deepseek]
name = "DeepSeek"
base_url = "https://api.deepseek.com/anthropic"
api_key = "sk-..."
model = "deepseek-v4-pro"

[providers.mimo]
name = "MiMo (Xiaomi)"
base_url = "https://api.xiaomimimo.com/anthropic"
api_key_env = "MIMO_API_KEY"
model = "mimo-v2.5-pro"
```

API keys can be stored directly or referenced via environment variable (`api_key_env = "VAR_NAME"`).

## Architecture

```
src/
├── main.rs       # CLI entry (clap)
├── config.rs     # TOML config read/write (~/.ccli/config.toml)
├── provider.rs   # Provider CRUD with presets
├── launcher.rs   # Claude Code process management, session binding, summary extraction
└── session.rs    # Session history, resume
```

## License

MIT
