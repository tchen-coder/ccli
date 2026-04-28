# ccli

[English](./README.md) | [中文](./README_CN.md)

为 Claude Code CLI 切换 LLM 提供商。

`ccli` 是一个轻量级封装工具，让 [Claude Code](https://docs.anthropic.com/en/docs/claude-code) 可以连接任意 Anthropic 兼容 API 的模型提供商，无需代理或协议转换。

## 工作原理

```
ccli use deepseek
    │
    ├─ 读取 ~/.ccli/config.toml 中的提供商配置
    ├─ 生成临时 settings.json（ANTHROPIC_BASE_URL + ANTHROPIC_AUTH_TOKEN）
    ├─ 生成 session UUID 用于确定性会话绑定
    └─ 启动: claude --bare --settings <path> --model <model> --session-id <uuid>
```

仅支持暴露 **Anthropic 兼容 Messages API** 端点的提供商，不做 OpenAI 到 Anthropic 的协议转换。

## 支持的提供商

| 提供商 | 端点 | 说明 |
|--------|------|------|
| Anthropic | `https://api.anthropic.com` | 原生 |
| DeepSeek | `https://api.deepseek.com/anthropic` | Anthropic 兼容 |
| MiMo (小米) | `https://api.xiaomimimo.com/anthropic` | Anthropic 兼容 |
| OpenRouter | `https://openrouter.ai/api/v1` | 多模型网关 |

支持自定义提供商，只要实现了 Anthropic Messages API。

## 安装

```bash
git clone <repo-url> && cd compatible_llm_cli
cargo install --path .
```

## 跨平台编译

```bash
# macOS（本地）
cargo build --release

# Linux x86_64
cargo build --release --target x86_64-unknown-linux-gnu

# Linux ARM64
cargo build --release --target aarch64-unknown-linux-gnu
```

### Docker 编译（推荐用于 Linux 目标）

无需本地交叉编译工具链，一条命令生成 Linux amd64 + arm64 静态链接二进制：

```bash
# 编译所有 Linux 目标 → dist/
make release-linux

# 本地 + Linux 目标 → dist/
make release-all

# 产出:
# dist/ccli-linux-amd64   (x86_64, musl 静态链接)
# dist/ccli-linux-arm64   (aarch64, musl 静态链接)
# dist/ccli-darwin-arm64  (macOS ARM 本地编译)
```

需要 Docker。Dockerfile 使用多阶段构建 + 依赖缓存，重复编译很快。

> **国内用户**: 建议配置 Docker Hub 镜像加速。Docker Desktop → Settings → Docker Engine 中添加：
> ```json
> { "registry-mirrors": ["https://docker.1ms.run"] }
> ```

## 使用

```bash
# 添加提供商（交互式，内置预设）
ccli llm add

# 查看已配置的提供商
ccli llm list

# 设置默认提供商
ccli llm set-default deepseek

# 使用指定提供商启动 Claude Code
ccli use deepseek

# 使用默认提供商启动
ccli

# 查看会话历史
ccli session list

# 恢复之前的会话
ccli session resume <id>
```

## 示例

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
# ... Claude Code 启动 ...

$ ccli session list
  [deepseek] deepseek-v4-pro
    +a1b2c3d4 2026-04-29T10:30  /Users/me/project  "实现用户认证中间件"
```

## 配置

配置文件: `~/.ccli/config.toml`

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

API key 可以直接存储，也可以通过环境变量引用（`api_key_env = "变量名"`）。

## 架构

```
┌─────────────────────────────────────────────────────────┐
│                       用户                               │
│                   ccli use <provider>                    │
└────────────────────────┬────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────┐
│                    ccli (Rust)                            │
│                                                         │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │ provider.rs  │  │ launcher.rs  │  │  session.rs   │  │
│  │ 提供商管理    │  │ 进程管理      │  │ 会话/恢复     │  │
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
          启动参数:         │  --bare --settings <path>
          环境覆盖:         │  ANTHROPIC_BASE_URL + ANTHROPIC_AUTH_TOKEN
          会话绑定:         │  --session-id <uuid> --model <model>
                           ▼
┌─────────────────────────────────────────────────────────┐
│            Claude Code CLI（完全不修改）                   │
│                                                         │
│  Claude Code 运行时完全不变。ccli 仅控制 Claude Code     │
│  连接哪个 API 端点和使用哪个凭证。所有 Claude Code 功能   │
│  — 工具、hooks、MCP、上下文、斜杠命令 — 与原生一致。     │
│                                                         │
│  Claude Code 升级完全透明：ccli 仅使用稳定的 CLI 参数    │
│  (--bare, --settings, --session-id, --resume)。          │
└────────────────────────┬────────────────────────────────┘
                         │
                         │  Anthropic Messages API
                         ▼
        ┌────────────────────────────────┐
        │       LLM 提供商端点            │
        │                                │
        │  ┌──────────┐  ┌───────────┐  │
        │  │ Anthropic │  │ DeepSeek  │  │
        │  └──────────┘  └───────────┘  │
        │  ┌──────────┐  ┌───────────┐  │
        │  │   MiMo   │  │OpenRouter │  │
        │  └──────────┘  └───────────┘  │
        │  ┌──────────────────────────┐ │
        │  │  任意 Anthropic 兼容端点  │ │
        │  └──────────────────────────┘ │
        └────────────────────────────────┘
```

```
src/
├── main.rs       # CLI 入口 (clap)
├── config.rs     # TOML 配置读写 (~/.ccli/config.toml)
├── provider.rs   # 提供商增删改查 + 预设
├── launcher.rs   # Claude Code 进程管理、会话绑定、摘要提取
└── session.rs    # 会话历史、恢复
```

## License

MIT
