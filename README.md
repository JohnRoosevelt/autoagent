# AutoAgent

一个用 Rust 从零实现的 Agent 学习项目。

本项目参考相关 Agent 教程的学习思路，但不以复刻文章代码为目标；会根据自己的理解逐步实现 LLM 调用、错误处理、流式输出、工具调用与 Agent 工作流等能力。

> 当前仍处于早期阶段：已经完成带上下文的 LLM 对话调用、配置加载、SSE 流式输出、最小 Agent Loop、Tool / Function Calling 协议承接、本地 Tool Registry 分发、最小 Agent 生命周期事件，以及基于消息预算的上下文裁剪、确定性 JSON 会话保存与恢复。

## 学习路线

项目按文章的 `00–39` 章节顺序推进。每章的实现由 `autoagent` 自己完成，章节状态和验收条件维护在 [ROADMAP.md](ROADMAP.md)。每章完成后的 Git checkpoint 与 Tag 使用方式见 [docs/checkpoints.md](docs/checkpoints.md)。

## 当前能力

- 使用 `reqwest` 向兼容 OpenAI Chat Completions 协议的服务发起请求；
- 默认使用 OpenRouter：`https://openrouter.ai/api/v1`；
- 默认模型为 `openrouter/free`，由 OpenRouter 从可用免费模型中路由；
- 通过环境变量配置模型服务、API Key 与模型名称；
- 对配置错误、网络错误、不可重试的 `4xx` 错误、可重试的 `408` / `429` / `5xx` 错误进行分类；
- 使用 `Role`、`Message` 与 `Conversation` 持有 system、user、assistant 消息历史，并将完整历史发送给模型；
- 解析服务端返回的输入与输出 token 用量，用三轮对话示例观察历史重放带来的输入增长；
- 通过 SSE 接收 OpenAI-compatible 服务的增量事件，并以 `AgentEvent::Model(StreamEvent)` 将 `Start`、文本增量与完成事件交给显示层；
- 由 `Agent` 管理会话、待处理输入与执行步数，在内部循环调用模型、转发流事件并回填 assistant 回复；
- 在 Agent 的模型调用边界按确定性有限退避重试网络、`408`、`429` 与 `5xx` 错误；失败尝试不会写入不完整 assistant 消息；
- 使用应用内协作式 `CancellationToken` 取消模型等待、流转发、退避与尚未开始的工具调用，并通过 `AgentEvent` 输出运行、输入、尝试、重试、账本、工具和终止生命周期；
- 声明 OpenAI-compatible `tools`，解析非流式与 SSE 流式 `tool_calls`（包括分块 arguments），并保留 assistant 工具调用上下文；
- 通过启动时组装的本地 `ToolRegistry` 注册、查找并串行分发编译进程序的 Rust 工具，拒绝重复工具名；
- 工具调用 arguments 在执行前解析为 JSON，并由工具完成最小字段校验；成功或失败结果均作为关联 `tool_call_id` 的 `role: tool` 消息回填，随后继续 Agent Loop；
- 以“没有待处理输入”或“达到最大模型调用步数”明确结束 Agent 运行；工具执行本身不额外计步。
- 可通过 `Agent::with_history_message_budget` 在每个模型调用边界限制发送历史消息数；system 消息始终保留，assistant `tool_calls` 与紧随的 tool 结果作为不可分割单元裁剪，并以 `AgentEvent::ContextTrimmed` 公开裁剪数量。
- 使用版本化、确定性 JSON 保存会话账本、待处理输入、可恢复执行配置与已完成运行报告；加载会拒绝未知版本，恢复时由宿主重新注入模型和本地工具；
- 提供 `scripts/run.sh`，避免每次手动输入环境变量。

## 项目结构

```text
.
├── src/
│   ├── main.rs          # 程序入口：配置演示输入并显示 Agent 生命周期与模型流事件
│   ├── agent.rs         # Agent Loop：会话状态、生命周期事件、重试/取消、上下文边界、模型流转发与回复回填
│   ├── context.rs       # Context Manager：按消息预算生成协议安全的模型请求历史
│   ├── message.rs       # Role、Message 与 Conversation 对话账本
│   ├── session.rs       # 版本化 JSON Session：捕获、保存、加载与恢复 Agent 状态
│   ├── tool.rs          # 本地 Tool trait、Registry 与固定离线演示工具
│   ├── llm.rs           # ChatResponse、Usage、StreamEvent 与 LLM 错误类型
│   └── llm/
│       └── client.rs    # OpenAI-compatible LLM HTTP 客户端
├── scripts/
│   └── run.sh           # 加载 .env 并启动项目
├── .env.example         # 环境变量模板
├── docs/
│   └── checkpoints.md   # 章节 checkpoint 与 Git Tag 使用说明
├── ROADMAP.md           # 与学习文章同步的章节计划与完成状态
└── Cargo.toml
```

## 环境要求

- Rust（建议使用当前 stable 工具链）
- 一个 OpenRouter API Key

## 配置

复制环境变量模板：

```sh
cp .env.example .env
```

编辑 `.env`，填入你的 OpenRouter API Key：

```dotenv
AGENT_BASE_URL=https://openrouter.ai/api/v1
AGENT_API_KEY=your_openrouter_api_key_here
AGENT_MODEL=openrouter/free
```

`AGENT_BASE_URL` 只填写 API 版本根路径，不要附加 `/chat/completions`；客户端会自行拼接该路径。

`.env` 已被 `.gitignore` 忽略，请不要提交 API Key。

## 运行

推荐使用启动脚本：

```sh
./scripts/run.sh
```

脚本会加载 `.env` 中的变量后执行：

```sh
cargo r -q
```

其中 `r` 是 Cargo 对 `run` 的内置简写，`-q` 会隐藏 Cargo 的常规构建日志，但不会隐藏程序输出或错误信息。

也可以手动设置环境变量后执行：

```sh
cargo run
```

## 当前示例

当前入口会创建一段包含 system 消息的会话，在启动时向本地 `ToolRegistry` 注册完全离线的 `get_weather` 工具，配置最多两次、初始 250ms 的确定性模型重试，并向 `Agent` 排入工具请求：

```text
system: 你是一个简洁、准确的助手。
user: 请查询北京现在的天气。请调用 get_weather，不要猜测结果。
```

`Agent` 在每一步才将下一条用户输入写入账本，并在每个模型调用边界按配置的消息预算生成请求历史；默认不限制。system 消息始终保留，assistant 工具调用及紧随的 tool 结果只会整体保留或整体移除，因此不会发送孤立的工具结果。账本本身不会被裁剪。Agent 再将请求历史和 Registry 导出的工具定义以 SSE 请求发送给模型。客户端把协议细节转换为 `StreamEvent::Start`、`TextDelta`、`Done`；Agent 以 `AgentEvent::Model` 包装这些模型流，而额外发出 started、input、attempt、retry、assistant recorded、tool start/finish 和终止生命周期事件。文本增量即时转发，而流式工具调用在客户端按 index 聚合，最终仅通过 `Done(ChatResponse)` 交付完整调用。入口会打印模型请求的调用 ID、函数名和原始 arguments。随后 Agent 将 assistant `tool_calls` 完整写回账本，经 Registry 按顺序执行工具，并将固定模拟天气结果（或清晰 JSON 错误）以 `role: tool` 与对应 `tool_call_id` 回填，继续请求模型生成最终回复。模型出现网络、408、429 或 5xx 失败时，Agent 会发出 `AgentEvent::Retry` 并重新发起该轮请求；只有收到完整成功的 `ChatResponse` 才入账。入口最后会显示成功回合、实际尝试和重试次数。

本章的 Registry 只是在程序启动时组装编译进二进制的本地 Rust 工具，不是动态库、WASM、目录扫描、热加载或 Plugin Manager。更进一步的动态扩展将分别在第 14 章 Skills、第 20 章 Plugins 与第 24 章 MCP 探索。

## 错误处理约定

- `main.rs` 使用 `anyhow::Result` 作为应用入口的统一错误出口；
- `agent` 模块用 `AgentError` 区分配置、模型、重试耗尽、模型任务与事件接收方错误；
- `RetryPolicy` 使用有限、无 jitter 的指数退避；`RunReport` 分别报告成功模型回合、实际尝试与重试次数；
- `CancellationToken` 是应用内协作式取消入口：取消会停止模型等待、流转发、退避和未开始的工具；已开始的同步本地工具不能被本章抢占；
- `llm` 模块使用 `thiserror` 定义 `LlmError`，让调用方可以依据错误类型决定是否重试；
- `408`、`429` 与 `5xx` 及网络错误被视为可重试错误；普通 `4xx`、配置与协议/JSON 解析错误不会重试；
- 缺失或为空的 `AGENT_API_KEY` 会被识别为配置错误，并在发起网络请求前返回。

## 后续方向

以下是计划逐步探索的方向，具体实现会随着学习和实践调整：

- 更完整的超时与限流处理；
- 对话历史与上下文管理；
- Tool Calling / Function Calling；
- 多步骤任务规划与执行；
- 更完善的日志、测试与可观测性。

## 第 10 章范围

本章提供 `Session`：以稳定字段顺序的版本化 JSON 保存完整会话消息、待处理输入、已用步数、重试与历史预算配置，以及已完成的 `RunReport`。`Session::restore` 只重建可序列化的 Agent 状态，模型与本地 `ToolRegistry` 必须由宿主进程重新注入，避免将密钥、运行时 channel 或不可重建的工具实例写入磁盘。本章不实现会话目录管理、加密、并发锁、自动保存或跨机器同步。

## 第 09 章范围

本章提供最小 `ContextManager`：在 Agent 的每次模型调用前按可配置的消息数量预算裁剪请求视图，不修改完整会话账本。system 消息优先保留；assistant `tool_calls` 和它们紧随的 tool 结果以完整单元保留或移除，避免破坏 OpenAI-compatible 协议。发生裁剪时会发出 `AgentEvent::ContextTrimmed { removed_messages }`。本章不实现摘要、额外模型调用、token 精确计数、向量数据库、长期记忆或持久化会话。

## 开发检查

```sh
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```
