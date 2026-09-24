# AutoAgent 最终架构

AutoAgent 是一个小型、可本地测试的 Rust Agent 学习项目。这里的“最终”指第 38 章确认现有实现已经形成的稳定边界，不表示项目已具备生产级服务能力。本文描述代码当前实际提供的契约；新功能应优先落在这些边界之一，而不是绕过它们。

## 运行链路

```text
CLI / embedding host
        |
        v
AgentBuilder -> App ------------------------> PermissionPolicy
        |                                       | (default deny)
        |                                       v
        |                                  Workspace + local tools
        v
Agent <-> Conversation -> ContextManager -> StreamChatModel -> OpenAI-compatible provider
  |          |                    |                  |
  |          |                    |                  +-- StreamEvent
  |          |                    +-- request view only
  |          +-- complete ledger
  |
  +-- ToolRegistry -> tool result -> Conversation
  +-- AgentEvent -> host renderer / local SessionActor forwarding
```

1. `main.rs` parses a command. UI-only commands end before configuration, credential validation, or network access.
2. The host loads non-secret `AppConfig`, creates `LlmClient`, explicitly grants its chosen capabilities, and composes `App` with `AgentBuilder`.
3. `App` checks `NetworkAccess`; `Agent` consumes pending input, derives a protocol-safe request view, calls `StreamChatModel`, and records only complete successful assistant responses.
4. When a response requests tools, `ToolRegistry` validates and runs the registered local tool, then the agent appends the associated `role: tool` result and continues the loop.
5. `AgentEvent` is the only lifecycle stream. A host renders it directly; `SessionActor` can forward it through a bounded local channel without creating a server.

## Stable boundaries

| Boundary | Primary code | Contract | Deliberate non-goal |
|---|---|---|---|
| Composition | `framework.rs` | `AgentBuilder` combines model, configuration, policy, workspace, tools, and the existing event lifecycle. | A replacement Agent API or another event bus. |
| Orchestration | `agent.rs` | Owns conversation, pending inputs, bounded model turns, retries, cancellation, tool-loop sequencing, and `AgentEvent`. | Global scheduling or preempting synchronous tools. |
| Model protocol | `llm.rs`, `llm/client.rs` | `StreamChatModel` hides provider transport behind streaming events and a final response. | Provider-specific orchestration policy. |
| State and request shaping | `message.rs`, `context.rs`, `session.rs`, `compaction.rs` | Keeps the complete conversation ledger separate from bounded request views and serializable session state. | Automatic summarization, durable distributed state, or secret persistence. |
| Local capabilities | `tool.rs`, `filesystem.rs`, `shell.rs`, `permission.rs` | Tools are compiled into a registry; workspace access is root-confined; sensitive capabilities require explicit approval. | Arbitrary shell, dynamic code loading, or implicit privilege escalation. |
| Optional host services | `service.rs`, `steering.rs`, `parallel.rs` | Bounded local channels, safe turn-boundary steering, and explicit-only bounded parallel tool execution. | Public transport, durable queues, or accidental concurrent workspace/network work. |
| Supporting seams | `memory.rs`, `skills.rs`, `artifact.rs`, `plugins.rs`, `mcp.rs`, `rag.rs`, `web.rs`, `guardrails.rs`, `cost.rs`, `observability.rs`, `evaluation.rs` | Small opt-in local contracts that can be composed by a host. | Claims of a full plugin ecosystem, remote MCP, vector store, web client, telemetry backend, or benchmark suite. |

## Dependency direction

- The host chooses configuration, provider credentials, workspace root, and approvals. Lower layers do not read or persist secrets.
- `Agent` depends on the `StreamChatModel` trait and `ToolRegistry`, not on `LlmClient` or CLI code. Tests use local model doubles at this seam.
- Context management creates model request views and never mutates the full ledger. Session persistence serializes recoverable state only; the host must reinject models and tools.
- Tool definitions flow from the registry to the model. Tool results return through the agent into the ledger with their tool-call IDs.
- Cross-cutting helpers remain opt-in. For example, `LocalRetriever`, `OutboundGuard`, and `TraceRecorder` do not silently alter requests, outputs, or control flow.

## Extension guidance

1. Add a provider by implementing `StreamChatModel`; preserve the existing stream/final-response contract.
2. Add a local tool through `Tool`, assign `ParallelSafe` only when its complete behavior is safe to run concurrently, and gate sensitive capability surfaces in the host composition.
3. Add a transport adapter outside the core loop. It should translate inbound messages to `SessionCommand` and consume forwarded `AgentEvent`s; it must define authentication, backpressure, cancellation, and ownership explicitly.
4. Keep persistence and network side effects at host-owned boundaries. Do not make an experimental helper automatically inject context or execute content.

## Why no new architecture module

The stable composition seam already exists as `AgentBuilder`/`App`, while `Agent`, `StreamChatModel`, `Tool`, and `SessionActor` are separately testable contracts. Adding a facade module now would duplicate those ownership boundaries without reducing coupling. This document records their intended use instead of introducing an abstraction with no new behavior.
