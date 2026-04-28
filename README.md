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
# macOS (local)
cargo build --release

# Linux x86_64
cargo build --release --target x86_64-unknown-linux-gnu

# Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu
```

### Docker (recommended for Linux targets)

Build static Linux binaries for amd64 and arm64 without a local cross-compilation toolchain:

```bash
# Build all Linux targets → dist/
make release-linux

# Build local (macOS) + Linux targets → dist/
make release-all

# Output:
# dist/ccli-linux-amd64   (x86_64, static musl)
# dist/ccli-linux-arm64   (aarch64, static musl)
# dist/ccli-darwin-arm64  (if built on macOS ARM)
```

Requires Docker. The Dockerfile uses multi-stage builds with dependency caching for fast rebuilds.

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
┌─────────────────────────────────────────────────────────┐
│                        User                             │
│                    ccli use <provider>                   │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│                     ccli (Rust)                          │
│                                                         │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ provider.rs  │  │ launcher.rs  │  │  session.rs   │  │
│  │ Profile CRUD │  │ Process Mgmt │  │ History/Resume│  │
│  └──────┬──────┘  └──────┬───────┘  └───────┬───────┘  │
│         │                │                   │          │
│         ▼                │                   │          │
│  ┌─────────────┐         │                   │          │
│  │  config.rs  │◄────────┤───────────────────┘          │
│  │ ~/.ccli/    │         │                              │
│  │ config.toml │         │                              │
│  └─────────────┘         │                              │
└──────────────────────────┼──────────────────────────────┘
                           │
          Spawns with:     │  --bare --settings <path>
          env override:    │  ANTHROPIC_BASE_URL + ANTHROPIC_AUTH_TOKEN
          session bind:    │  --session-id <uuid> --model <model>
                           ▼
┌─────────────────────────────────────────────────────────┐
│              Claude Code CLI (unchanged)                 │
│                                                         │
│  The entire Claude Code runtime remains untouched.      │
│  ccli only controls which API endpoint and credentials  │
│  Claude Code connects to. All Claude Code features —    │
│  tools, hooks, MCP, context, slash commands — work      │
│  exactly as they do natively.                           │
│                                                         │
│  Upgrades to Claude Code are fully transparent:         │
│  ccli adapts automatically since it only uses stable    │
│  CLI flags (--bare, --settings, --session-id, --resume).│
└────────────────────────┬────────────────────────────────┘
                         │
                         │  Anthropic Messages API
                         ▼
        ┌────────────────────────────────┐
        │      LLM Provider Endpoint     │
        │                                │
        │  ┌──────────┐  ┌───────────┐  │
        │  │ Anthropic │  │ DeepSeek  │  │
        │  └──────────┘  └───────────┘  │
        │  ┌──────────┐  ┌───────────┐  │
        │  │   MiMo   │  │OpenRouter │  │
        │  └──────────┘  └───────────┘  │
        │  ┌──────────────────────────┐ │
        │  │  Any Anthropic-compat.   │ │
        │  └──────────────────────────┘ │
        └────────────────────────────────┘
```

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
