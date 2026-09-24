# AutoAgent

一个用 Rust 从零实现的 Agent 学习项目。

本项目参考相关 Agent 教程的学习思路，但不以复刻文章代码为目标；会根据自己的理解逐步实现 LLM 调用、错误处理、流式输出、工具调用与 Agent 工作流等能力。

> 当前仍处于早期阶段：已经完成带上下文的 LLM 对话调用、配置加载、SSE 流式输出、最小 Agent Loop、Tool / Function Calling 协议承接、本地 Tool Registry 分发、最小 Agent 生命周期事件，以及基于消息预算的上下文裁剪、确定性 JSON 会话保存与恢复，受工作区根目录约束的文件工具，以及离线、allowlist 的 Cargo/Git 检查工具，最小 CLI/slash command 入口，以及受根目录约束、按需加载的 Markdown Skills 定义，以及具有明确 context 边界的最小 Subagent wrapper，以及显式分级 Markdown Memory 注入，以及外置大对象的 Artifact refs，以及工具调用前后的 Hooks，以及模型调用边界 Middleware，以及 manifest/registry-only Plugins convention，以及 defaults < local file < environment 的非私密 Configuration，以及 default-deny Permission approvals，以及拒绝 untrusted execution 的 declarative Sandbox profile，以及 local-test-double-only MCP JSON-RPC protocol，以及不改写账本的 local Compaction request view，以及不执行任务的 in-memory Job scheduler，以及 provider-agnostic Model Router candidates，以及 local structured Observability spans，以及 repeatable local Evaluation cases，以及以 `AgentBuilder`/`App` 组合配置、权限、工作区工具和既有事件流的最小 Framework，以及默认串行、仅显式 `ParallelSafe` 工具可通过有界 executor 并发的安全并行分发，以及在安全回合边界应用的有界 Steering inbox，以及本地稳定前缀与 token/cached-token 成本核算，以及将外部内容作为数据的出站 secret redaction guardrail，以及无真实外网请求的 safe Web Fetch policy/parser，以及确定性本地 RAG-style retrieval，以及无网络监听的 local Agent session actor skeleton。

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
- 在 canonical workspace root 内列出、读取、创建和显式覆盖 UTF-8 文件；拒绝绝对路径、`..` traversal 和经 symlink 逃逸的已有目标，且不提供删除/重命名；
- `run_inspection` 仅运行离线 `cargo check`/`cargo test` 与只读 `git status`/`diff`/`log`；不接受 shell 或任意命令；
- 提供依赖无关的 CLI：`/help`、`/status`、`/exit`/`/quit`，或把普通位置参数作为一次 Agent prompt；UI-only 命令不会加载 LLM 配置；
- 提供 `scripts/run.sh`，避免每次手动输入环境变量。

## 项目结构

```text
.
├── src/
│   ├── main.rs          # 程序入口：配置演示输入并显示 Agent 生命周期与模型流事件
│   ├── agent.rs         # Agent Loop：会话状态、生命周期事件、重试/取消、上下文边界、模型流转发与回复回填
│   ├── command.rs       # 最小 CLI/slash command 解析
│   ├── context.rs       # Context Manager：按消息预算生成协议安全的模型请求历史
│   ├── filesystem.rs    # 受 canonical workspace root 约束的文件访问
│   ├── message.rs       # Role、Message 与 Conversation 对话账本
│   ├── session.rs       # 版本化 JSON Session：捕获、保存、加载与恢复 Agent 状态
│   ├── shell.rs         # allowlist 的离线 Cargo / 只读 Git 工作区检查
│   ├── skills.rs        # root-confined 的按需 Markdown Skills 定义
│   ├── subagent.rs      # 显式 task/context 边界的同步 child wrapper
│   ├── memory.rs        # root-confined 的显式分级 Markdown Memory
│   ├── artifact.rs      # root-confined 的外置 bytes 与小引用
│   ├── hooks.rs         # Tool pre/post local callbacks
│   ├── middleware.rs    # StreamChatModel call wrapper
│   ├── plugins.rs       # manifest/registry-only Plugins convention
│   ├── configuration.rs # defaults < file < environment 的非私密配置
│   ├── permission.rs    # default-deny 的敏感能力审批
│   ├── sandbox.rs       # declarative profile；不执行不可信代码
│   ├── mcp.rs           # JSON-RPC protocol；仅 local test double
│   ├── compaction.rs    # local summary replacement request view
│   ├── scheduler.rs     # in-memory job state；不执行 worker
│   ├── model_router.rs  # stable primary/fallback candidates
│   ├── observability.rs # local structured span recorder
│   ├── evaluation.rs    # repeatable injected local case harness
│   ├── framework.rs     # AgentBuilder/App：配置、策略、工作区工具与既有事件流的最小组合
│   ├── parallel.rs      # explicit ParallelSafe 工具的有界并发 dispatch
│   ├── steering.rs      # local steering inbox 与协作式取消
│   ├── cost.rs          # stable prefix fingerprint 与本地 token 成本核算
│   ├── guardrails.rs    # outbound untrusted-content 与 secret filtering
│   ├── web.rs           # strict allowlist/size safe-fetch policy 与本地 HTML text
│   ├── rag.rs           # deterministic local token-overlap retrieval
│   ├── service.rs       # local bounded-channel Agent session actor skeleton
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

### 最小命令

```sh
cargo run -- /help
cargo run -- /status
cargo run -- "解释这个项目的结构"
```

`/help`、`/status`、`/exit` 和 `/quit` 都在创建 LLM client 前处理；未知 slash command 会返回明确错误。此阶段是最小终端入口，而非全屏 TUI。

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

## 第 37 章范围

`SessionActor`/`SessionClient` 提供同进程、有界 channel 的 prompt/cancel 命令与既有 `AgentEvent` 转发边界，供未来 transport adapter 使用。它不启动 HTTP、SSE、websocket 或任何公开网络监听器。

## 第 36 章范围

`LocalRetriever` 对调用方提供的文档和 query 做确定性、大小写无关的 token overlap 排序。它是可离线测试的最小检索边界，不生成真实 embedding、不使用 vector database，也不自动注入模型上下文。

## 第 35 章范围

`FetchPolicy` 只验证 HTTPS exact-host allowlist URL，拒绝 credential、显式 port 和常见 SSRF 地址形状；本地 HTML 转文本还受字节上限约束。本章不做真实 GET、重定向、DNS 或 IP 解析。

## 第 34 章范围

`OutboundGuard` 将外部文本视为数据：它标记常见提示注入短语，并在宿主输出前替换显式配置的 secret literals。它不执行外部内容、不尝试自动泄漏检测，也不把原始 secret 写入日志。

## 第 33 章范围

`StablePrefix` 仅为调用方指定的消息生成本地确定性 fingerprint；`TokenCost` 以明确每 token 微单位 pricing 核算输入、已缓存输入和输出成本。它不发送 provider-specific cache 指令，也不将估算误报成远端缓存命中。

## 第 32 章范围

`SteeringInbox` 是本地、有界的控制入口。插话文本保持顺序，并只在宿主决定的安全回合边界取出；取消复用已有协作式 `CancellationToken`，不会抢占已经开始的同步工具。它不是全局消息总线、持久化队列或网络 API。

## 第 31 章范围

`ToolExecution` 默认 `Serial`。`execute_safe_batch` 仅为明确声明 `ParallelSafe` 的工具使用有界 `spawn_blocking`，并保持调用结果的原始顺序；工作区、命令和网络工具不会因本章意外变成并发工具。它不改变既有 Agent Loop 的串行协议，也不实现后台队列。

## 第 30 章范围

`AgentBuilder`/`App` 将经过验证的 `AppConfig`、`PermissionPolicy`、root-confined `Workspace` 工具和既有 `AgentEvent` lifecycle 组合为宿主入口。默认仅注册天气、列目录和读文件工具；文件写入、工作区 inspection 与模型网络运行必须显式批准。它不替换现有 `Agent` API、不新建 event bus，也不实现动态插件。

## 第 29 章范围

`evaluate` 使用注入的本地实现运行可重复的输入/预期输出 case，并报告逐项通过情况。它不调用模型、网络或外部 benchmark。

## 第 28 章范围

`TraceRecorder` 在内存中按记录顺序保存名称和结构化字段，供宿主检查。没有遥测导出、日志框架或指标后端。

## 第 27 章范围

`ModelRouter` 验证并输出 primary 后 fallback 的稳定候选顺序。具体 provider 调用、错误分类和是否降级均仍由宿主决定。

## 第 26 章范围

`JobScheduler` 提供稳定 ID 的内存 job enqueue、查询和 cancel 状态转换。它不创建 worker、执行任务、持久化或恢复任务。

## 第 25 章范围

`Compactor` 使用调用方给定的摘要将旧消息替换为一个 system summary，并保留最近消息。它只创建请求视图，不调用模型也不修改完整会话账本。

## 第 24 章范围

`mcp` 定义 manifest、JSON-RPC 2.0 `initialize` 消息和可注入 transport trait。测试只使用内存 local double；不连接网络、不启动外部 MCP server。

## 第 23 章范围

`SandboxProfile` 只声明只读、禁网的未来 OS/container 边界。`execute_untrusted` 一律返回 unavailable：本项目没有执行不可信代码、启动容器或提升权限。

## 第 22 章范围

`PermissionPolicy` 默认拒绝文件写入、删除、命令执行和网络访问。宿主必须逐项明确授权；该关卡只返回决定，不直接执行任何操作。

## 第 21 章范围

`AppConfig` 以显式 defaults < local file < environment 顺序合并 model、base URL 与步数等非私密设置。API key 保持在进程环境中，不会由该模块读取、持久化或打印。

## 第 20 章范围

`PluginManifest` 和 `PluginRegistry` 只约定可选、已链接能力的稳定 metadata 注册。它不扫描插件目录、不加载动态库或 WASM，也没有热加载。

## 第 19 章范围

`MiddlewareModel` 在既有 `StreamChatModel` 调用前执行 `ModelMiddleware`，保留原模型的流式协议。它不是 middleware 链或 Agent 重写框架。

## 第 18 章范围

`HookedTool` 是普通 `Tool` 的最小 wrapper：`before` 可拒绝调用，`after` 接收成功或失败结果。不执行用户脚本，不建立全局事件系统。

## 第 17 章范围

`ArtifactStore` 将调用者提供的 bytes 以 create-new 文件外置，只返回名称和大小的 `ArtifactRef`，避免大数据进入上下文。无数据库或远程存储。

## 第 16 章范围

`MemoryStore` 仅按调用者传入的 scope 顺序读取 root-confined Markdown 文件并返回给 context 边界。它不自动发现、检索、写入或使用数据库。

## 第 15 章范围

本章提供同步 `Subagent` wrapper。它仅把调用者显式提供的任务和复制后的上下文传给 `ChildAgent`，不共享 parent Agent、会话或工具，不提供并行执行。

## 第 14 章范围

本章增加 `SkillDirectory`，将任务说明作为受 canonical root 约束的 UTF-8 Markdown 文件，以安全名称按需列出和加载。Skill 是供宿主选择注入的纯文本知识，不会执行文件内容、递归发现工作区内容或加载代码。

## 第 13 章范围

本章增加轻量 `command` 模块，不引入 CLI 或 TUI 依赖。它识别 `/help`、`/status`、`/exit`、`/quit`，并将非 slash 位置参数合并为一条 prompt；默认仍保留天气演示 prompt。UI-only 命令在读取 `AGENT_API_KEY` 前返回，所以可安全地用于本地帮助和状态确认。本章不实现全屏 TUI、交互式行编辑、历史、多会话菜单或后台输入。

## 第 12 章范围

本章提供 `run_inspection`：命令名只能是 `cargo_check`、`cargo_test`、`git_status`、`git_diff` 或 `git_log`。实现直接执行固定 program/arguments，而非 shell；Cargo 命令附加 `--offline`，Git 命令均为只读检查。任何其他命令（特别是 `git reset`、`git clean`、`rm`、网络工具或带参数的任意 shell 命令）都会被拒绝。本章不实现终端、任意进程执行、命令审批或网络访问。

## 第 11 章范围

本章以 `Workspace` 封装本地文件访问，并注册 `list_files`、`read_file`、`create_file`、`overwrite_file`。所有路径必须相对配置的 canonical 工作区根目录，拒绝绝对路径和 `..`，读/写时重新检查 canonical 目标或父目录以阻止常见 symlink 逃逸。创建操作不会覆盖已有文件；覆盖是单独、显式的工具。本章不提供删除、重命名、递归扫描、文件权限变更或工作区外访问。

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
