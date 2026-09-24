# AutoAgent

A small Rust learning project that builds an OpenAI-compatible, tool-using agent from explicit local components. It follows the chapter order of a related Agent tutorial series, but the implementation, API, and scope are AutoAgent’s own. **All chapters 00–39 are complete.**

- Architecture: [docs/architecture.md](docs/architecture.md)
- Mature-design comparison: [docs/mature-project-comparison.md](docs/mature-project-comparison.md)
- Roadmap and acceptance checkpoints: [ROADMAP.md](ROADMAP.md)
- Tagged chapter checkpoints: [docs/checkpoints.md](docs/checkpoints.md)

## What works today

- OpenAI-compatible Chat Completions over `reqwest`, including SSE text/tool-call aggregation and parsed usage.
- An `Agent` loop with bounded model turns, deterministic retry/backoff for retryable failures, cooperative cancellation, lifecycle events, and complete conversation recording.
- Compiled-in tool registration and OpenAI-compatible tool results; default composition exposes weather plus root-confined list/read workspace tools.
- Explicit capability approvals for file writes, workspace inspections, and model network access; workspace paths reject absolute paths, traversal, and common symlink escapes.
- Request-history budgeting that preserves system messages and complete tool exchanges, plus deterministic JSON session capture/restore (with models and local tools reinjected by the host).
- Local, opt-in building blocks for skills, memory, artifacts, hooks, middleware, plugin metadata, MCP protocol doubles, compaction views, scheduling state, model candidates, tracing, evaluation, guarded text, safe-fetch policy, retrieval, parallel-safe tool batches, steering, cost estimates, and a bounded local session actor.

See [docs/architecture.md](docs/architecture.md) for exact ownership and data flow. Several of these later building blocks are intentionally local contracts rather than end-to-end product features.

## Fast, safe start

### Prerequisites

- Current stable Rust toolchain.
- An API key for an OpenAI-compatible provider (OpenRouter is the default example).

### Offline commands first

These commands do not load credentials or make a model request:

```sh
cargo run -- /help
cargo run -- /status
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```

### Configure and run a prompt

```sh
cp .env.example .env
# edit .env; keep the key private
cargo run -- "Explain this repository's architecture briefly"
```

The supplied `scripts/run.sh` loads `.env` and runs the default demonstration prompt:

```sh
./scripts/run.sh
```

`/help`, `/status`, `/exit`, and `/quit` are handled before LLM configuration. Any other slash command is rejected. Do not commit `.env`.

## Configuration

The executable loads non-secret settings in this order: defaults, an optional local settings file when a host supplies one, then process environment. The current CLI uses environment values directly.

| Variable | Default | Purpose |
|---|---|---|
| `AGENT_API_KEY` | none | Required provider credential; read by the HTTP client and never persisted by `AppConfig`. |
| `AGENT_BASE_URL` | `https://openrouter.ai/api/v1` | API version root. Do not append `/chat/completions`. |
| `AGENT_MODEL` | `openrouter/free` | Model identifier sent to the compatible provider. |
| `AGENT_MAX_STEPS` | `3` | Positive maximum successful model turns per agent run. |

A minimal `.env` is:

```dotenv
AGENT_BASE_URL=https://openrouter.ai/api/v1
AGENT_API_KEY=your_api_key_here
AGENT_MODEL=openrouter/free
AGENT_MAX_STEPS=3
```

## Architecture map

| Layer | Key modules | Responsibility |
|---|---|---|
| Host and composition | `main.rs`, `framework.rs`, `command.rs` | Parses commands, selects explicit permissions, and assembles an app. |
| Agent core | `agent.rs`, `message.rs`, `context.rs`, `session.rs` | Owns loop state, full ledger, request views, retries, cancellation, and events. |
| Provider boundary | `llm.rs`, `llm/client.rs` | Hides HTTP/SSE behind `StreamChatModel`, `StreamEvent`, and a final response. |
| Capability boundary | `tool.rs`, `filesystem.rs`, `shell.rs`, `permission.rs` | Registers local tools and constrains filesystem/inspection authority. |
| Local supporting seams | `service.rs`, `parallel.rs`, `steering.rs`, `rag.rs`, `guardrails.rs`, and related modules | Offer opt-in, bounded helpers without silently changing the main loop. |

The dependency and extension rules are documented in [docs/architecture.md](docs/architecture.md). `AgentEvent` is the single lifecycle stream; `SessionActor` forwards it locally and does not run an HTTP, SSE, or WebSocket server.

## Safety and limitations

AutoAgent is a teaching project, not a hardened production agent service.

- Network model execution is capability-gated in `App`, but the current CLI explicitly approves network access, file writes, and workspace inspection for its demonstration flow. Embed the framework with a narrower `PermissionPolicy` for safer use.
- Filesystem tools are root-confined and inspection commands are fixed/offline/readonly. There is no arbitrary shell tool, delete operation, dynamic plugin loading, or untrusted-code execution.
- Cancellation is cooperative. It stops model waits, forwarding, retry backoff, and not-yet-started tools; it does not preempt a synchronous tool already running.
- Web fetch validates URLs and converts caller-provided local HTML; it does not make real GET requests, resolve DNS, or follow redirects. Retrieval is deterministic token overlap, not embeddings or a vector database.
- Prompt-cache fingerprints and token costs are local calculations, not provider cache controls or billing records. Guardrails mark common instruction-like text and redact configured literals; they are not a comprehensive security system.
- The session actor is an in-process bounded-channel skeleton. It provides no listener, authentication, multi-tenant isolation, durable queue, transport protocol, or server lifecycle.
- Session JSON intentionally excludes credentials, model clients, channels, and runtime tool instances. A host restores state and reinjects those dependencies.

## Development checks

Run the complete local quality gate before a checkpoint:

```sh
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```

## Chapter status and checkpoints

The 00–39 learning path is complete. Chapter 38 records the final architecture and stable boundaries; Chapter 39 compares those boundaries with mature designs using linked primary sources and clearly labeled inferences. See [ROADMAP.md](ROADMAP.md) for each chapter’s scope and [docs/checkpoints.md](docs/checkpoints.md) to inspect or branch from tagged snapshots.

```sh
# List completed chapter snapshots
git tag --list 'chapter-*'

# Inspect one snapshot
git show chapter-39-mature-project-comparison

# Experiment from a snapshot without changing it
git switch -c my-experiment chapter-39-mature-project-comparison
```

Tags are immutable learning checkpoints. Avoid committing directly from a detached tag checkout; create a branch first.
