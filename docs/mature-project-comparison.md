# Chapter 39: Mature project comparison

This is a design comparison, not a feature scorecard or a claim that AutoAgent is production-equivalent to any project below. Statements in the **Documented focus** column are limited to the linked primary documentation as accessed on 2026-09-24. The **Lesson for AutoAgent** column is an explicit architectural inference, not an assertion about another project’s internals.

## Baseline

AutoAgent currently has a deliberately small core: a host composes `App` through `AgentBuilder`; `Agent` owns a bounded tool-calling loop and complete conversation ledger; `StreamChatModel` isolates model transport; tools are compiled into a local registry; and `AgentEvent` is the lifecycle stream. Its surrounding modules are mostly opt-in local contracts. The detailed boundary map is in [architecture.md](architecture.md).

## Comparison

| Design / product | Documented focus | Relevant AutoAgent boundary | Lesson for AutoAgent |
|---|---|---|---|
| [Rig](https://docs.rs/rig-core/latest/rig_core/) | Rust library focused on ergonomic, modular LLM applications. Its docs describe provider-agnostic completion/embedding contracts, portable tools, conversation-memory contracts, vector-store interfaces, and a separate classic agent runtime. | `StreamChatModel`, `Tool`, `LocalRetriever`, session/context seams. | Keep provider, tool, memory, and retrieval contracts independently replaceable. AutoAgent already has a narrow model/tool seam, but its local retriever and session mechanism should remain opt-in rather than being silently coupled into the loop. |
| [LangGraph](https://docs.langchain.com/oss/python/langgraph/overview) | Low-level orchestration/runtime for long-running stateful agents; the overview emphasizes graphs that mix deterministic and LLM-driven steps, persistence, streaming, and human-in-the-loop. | `Agent` state machine, `SessionActor`, `SteeringInbox`, scheduler/session state. | A single linear loop is appropriate for the current project. If workflows need branching, durable resumptions, or operator intervention, model the state transitions explicitly instead of extending `Agent` with ad-hoc flags. |
| [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/) | A higher-level SDK with agents, tools, handoffs, guardrails, sessions, tracing, evaluation, and a managed run loop. | `Agent`, subagent wrapper, guardrails, observability, evaluation, session. | It demonstrates how a small vocabulary can compose a richer runtime. AutoAgent should keep its current explicit host ownership unless it intentionally takes responsibility for managed handoffs, persistent sessions, and operational tracing. |
| [OpenCode](https://opencode.ai/docs/) | Open-source AI coding agent available in terminal, desktop, and IDE surfaces; its introduction documents project initialization, provider configuration, plan/build modes, and undo/redo. | `command.rs`, workspace tools, future presentation/transport adapters. | Coding-agent UX needs an explicit review/plan boundary and change recovery, not merely more tools. Those are product workflow requirements; they do not follow automatically from AutoAgent’s current local tool registry. |
| [Codex](https://developers.openai.com/codex/) | OpenAI’s documentation navigation describes Codex across CLI, IDE extension, cloud, local environments, worktrees, permissions, sandboxing, skills, MCP, and hooks. | host composition, permissions, sandbox declaration, skills/MCP/hooks, `SessionActor`. | A usable coding agent separates execution surface, environment, permissions, and extension points. AutoAgent has early seams for these concerns but no transport, sandbox executor, or worktree lifecycle; documentation must not imply otherwise. |
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) | Agentic coding tool for terminal, IDE, desktop, and browser; its overview documents codebase editing, command execution, MCP, instructions/skills/hooks, parallel agents, and remote or scheduled workflows. | workspace/inspection tools, skills, hooks, subagent, parallel execution, service boundary. | The product perspective reinforces that capability controls and user-visible review matter as much as an agent loop. AutoAgent’s conservative serial default and root-confinement are sound foundations, but it lacks the product/runtime layers documented there. |
| [Pi](https://github.com/earendil-works/pi) | The project describes a coding-agent CLI, agent runtime with tool calling and state management, unified multi-provider LLM API, telemetry contracts, and a separate durable runtime. Its README also states that Pi does not include built-in permissions restricting filesystem, process, network, or credential access. | model abstraction, `Agent`, observability, session/durability, `PermissionPolicy`. | Permission behavior must be conspicuous and host-owned. AutoAgent’s default-deny policy is a useful contrast, but the CLI’s explicit broad approvals mean an embedding host—not the policy type alone—determines the effective safety posture. |

## What should remain stable

The comparison supports preserving these boundaries rather than replacing them with a catch-all framework layer:

1. **Host-owned authority:** credentials, workspace root, capability grants, and any future transport authentication belong outside `Agent`.
2. **Explicit orchestration state:** `Agent` remains the right home for the present bounded sequential loop. A graph/runtime should be introduced only for a concrete branching or durable-workflow requirement.
3. **Replaceable protocol seams:** provider calls stay behind `StreamChatModel`; local actions stay behind `Tool`; lifecycle reporting stays on `AgentEvent`.
4. **Opt-in supporting capabilities:** retrieval, memory, guardrails, evaluation, tracing, and service adapters must not change model inputs or authority implicitly.
5. **Honest product boundary:** UI, multi-session management, remote execution, sandbox enforcement, durable jobs, approvals, and operational backends are future product work, not features supplied by the current local skeletons.

## Sources and maintenance

- Rig core crate documentation: <https://docs.rs/rig-core/latest/rig_core/>
- LangGraph overview: <https://docs.langchain.com/oss/python/langgraph/overview>
- OpenAI Agents SDK overview: <https://openai.github.io/openai-agents-python/>
- OpenCode introduction: <https://opencode.ai/docs/>
- Codex developer documentation: <https://developers.openai.com/codex/>
- Claude Code overview: <https://docs.anthropic.com/en/docs/claude-code/overview>
- Pi repository README: <https://github.com/earendil-works/pi>

These products change independently. Before using this document to make an integration or security decision, revisit the linked primary source and the relevant versioned API/security documentation. This comparison intentionally does not infer licensing, pricing, implementation architecture, security guarantees, or feature parity beyond what the listed sources state.
