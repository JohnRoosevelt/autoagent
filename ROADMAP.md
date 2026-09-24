# AutoAgent 路线图

`autoagent` 参考《Rust 构建 AI Agent 系列》的章节顺序推进，以便随时对照文章理解每个组件的来源、问题与设计取舍。

这是一份**同步学习路线图，不是代码复刻清单**：章节主题、推进顺序和验收目标与文章对齐；具体模块组织、命名、Provider 与实现细节由 `autoagent` 自己决定。

来源文章：[Rust构建AI Agent系列：00 · 先拆零件，再造轮子：到底在造什么](https://mp.weixin.qq.com/s/c_Q5Is1mY0WZ3qHBqVGYvw)

## 状态说明

- `☐` 未开始
- `◐` 进行中
- `☑` 已完成
- `⊘` 暂不做或改用其他方案（必须记录原因）

只有代码已实现、可以运行，并完成相应检查后才标记 `☑`。

---

## 第零幕 · 项目地基

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☑ | 00 | 先拆零件，再造轮子：到底在造什么 | 明确 Agent = 模型 + 工具 + 循环 + 状态；确定项目目标、开发方式与路线图。 |

## 第一幕 · 核心循环（01–08）

目标：先造出一个能调用模型、能使用工具、能观察执行过程的最小 Agent 核心。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☑ | 01 | LLM API | 用 HTTP POST + JSON 调用 OpenAI-compatible 模型服务。已完成 OpenRouter 真实调用验证、错误分类与离线响应解析测试。 |
| ☑ | 02 | Message / Token / Context | 建立消息历史模型，理解上下文窗口与 token 预算。 |
| ☑ | 03 | Streaming / SSE | 解析 OpenAI-compatible SSE 增量事件，实时输出文本并在结束后汇总回复与 usage。 |
| ☑ | 04 | Agent Loop | 以独立 Agent 管理会话、状态与流式调用循环；通过待处理输入和最大步数保护明确结束运行，并为后续工具结果回填预留分支。 |
| ☑ | 05 | Tool / Function Calling | 建立 OpenAI-compatible tools/tool_calls 协议模型、请求序列化、非流式与 SSE 解析及 Agent 暂停承接；不执行真实工具。 |
| ☑ | 06 | Tool Registry | 建立启动时组装的本地工具注册表、最小参数校验、串行分发、`role: tool` 结果回填与继续循环；不是动态插件系统。 |
| ☑ | 07 | Retry / Cancel | 在 Agent 模型边界实现有限、确定性 retry/backoff 与应用内协作式取消；可观察尝试/重试/取消，不实现全局限流、熔断、队列或完整 Ctrl+C 集成。 |
| ☑ | 08 | Event / Lifecycle | 以 `AgentEvent` 描述运行、输入、尝试、重试、账本、工具与终止生命周期；保留 `StreamEvent` 作为 LLM 流。 |

## 第二幕 · 让它活着（09–13）

目标：让 Agent 能够管理长上下文、恢复会话，并在代码工作区中完成实际任务。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☑ | 09 | Context Manager | 在模型调用边界按消息预算裁剪请求历史，保留 system 与完整 tool-call 交换。 |
| ☑ | 10 | Session / Resume | 将历史、状态和执行记录持久化；进程重启后可以恢复会话。 |
| ☑ | 11 | Filesystem 工具 | 安全地列出、读取、创建和修改工作区文件。 |
| ☑ | 12 | Shell / Git 工具 | 运行受控命令、查看测试结果与 Git 状态，为 coding agent 提供手和脚。 |
| ☑ | 13 | Command / TUI | 增加斜杠命令与终端交互界面。 |

## 第三幕 · 生长（14–20）

目标：让 Agent 具备可扩展、可组合、可定制的能力。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☑ | 14 | Skills | 用受根目录约束、按名称按需加载的 Markdown 说明沉淀任务知识和工作流程。 |
| ☑ | 15 | Subagent | 将具有显式任务与复制上下文边界的子 Agent 包装为同步委托。 |
| ☑ | 16 | Memory | 建立受根目录约束、由调用方显式选择的分级 Markdown 长期记忆注入。 |
| ☑ | 17 | Artifact | 将大对象外置到受根目录约束的磁盘文件，上下文只保留引用。 |
| ☑ | 18 | Hooks | 以最小 wrapper 在工具调用前后执行本地定义的回调。 |
| ☑ | 19 | Middleware | 用最小 model wrapper 在模型调用前包装既有 Agent 模型边界。 |
| ☐ | 20 | Plugins | 定义插件目录与注册约定，将可选能力打包和加载。 |

## 第四幕 · 上生产（21–29）

目标：补齐配置、安全、可观测性与评测，让 Agent 具备可靠运行的基础。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☐ | 21 | Configuration | 实现默认值 < 文件 < 环境变量的分层配置；私密信息不进入仓库。 |
| ☐ | 22 | Permission | 对文件写入、删除、命令执行和网络访问建立审批关卡。 |
| ☐ | 23 | Sandbox | 通过操作系统级或容器级隔离限制 Agent 的实际能力边界。 |
| ☐ | 24 | MCP | 使用 JSON-RPC 接入跨进程的外部能力与工具服务。 |
| ☐ | 25 | Compaction (`/compact`) | 在上下文装不下时，用摘要替换旧历史并保留任务关键事实。 |
| ☐ | 26 | Job / Scheduler | 将长任务放入后台队列执行，并查询、取消或恢复任务。 |
| ☐ | 27 | Model Router | 在多个模型供应商之间选路、降级和回退。 |
| ☐ | 28 | Observability | 用 trace、span、日志、指标记录 Agent 的执行过程。 |
| ☐ | 29 | Evaluation | 为 Agent 行为建立可重复执行的任务评测与回归测试。 |

## 第五幕 · 框架抽象（30）

目标：只抽取已经多次出现、边界已经稳定的重复能力。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☐ | 30 | Framework | 将经过实践验证的模型、工具、循环、状态与事件能力抽象为框架。 |

## 第六幕 · 再生长（31–37）

目标：补齐真实 Agent 产品中常见的并发、成本、安全、网络和服务化能力。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☐ | 31 | Parallel Tool Use | 声明工具并发安全性，并在 Agent Loop 中并行分发独立调用。 |
| ☐ | 32 | Steering | 支持运行中插话、收件箱、回合边界与安全取消。 |
| ☐ | 33 | Prompt Caching / Cost | 保持稳定前缀以利用缓存，并记录 token、缓存与模型成本。 |
| ☐ | 34 | Guardrails | 将外部内容视为数据而非指令，并对密钥和敏感内容做出站过滤。 |
| ☐ | 35 | Web Fetch | 实现安全 HTTP GET、HTML 转文本、截断和 SSRF 防护。 |
| ☐ | 36 | RAG / Embedding | 将文本转为向量，检索相近内容，并作为普通工具返回结果。 |
| ☐ | 37 | Agent as Service | 将会话实现为 actor，并通过 SSE 等方式向客户端推送事件。 |

## 终幕 · 收官与对照（38–39）

目标：完成最终架构，理解成熟产品和框架的设计选择。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☐ | 38 | 最终架构 | 汇总并整理完整架构、模块边界与运行链路。 |
| ☐ | 39 | 成熟项目对照 | 与 Rig、LangGraph、OpenAI Agents SDK、OpenCode、Codex、Claude Code、pi 等进行设计对照。 |

---

## 第 01 章 Checkpoint

- ☑ 使用 `.env` 中的 OpenRouter 配置完成一次真实模型调用；
- ☑ 正确分类成功响应、网络失败、`4xx`、`429` 与 `5xx`；
- ☑ 为错误分类与响应解析补充不依赖真实 API 的测试；
- ☑ `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过；

## 第 02 章 Checkpoint

- ☑ 定义 `Role` 与 `Message`，用于表示 system、user、assistant 三类消息；
- ☑ 建立 `Conversation`，统一持有并追加消息历史；
- ☑ 将当前的一次性 `chat_raw(&str)` 调整为接收消息列表；
- ☑ 以离线请求体测试完成两轮对话验证，确认第二轮请求包含第一轮上下文；
- ☑ 解析服务端 usage，并以三轮示例展示完整历史重放导致的输入 token 增长；
- ☑ 为消息转换、对话历史与 usage 解析补充单元测试；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过；

## 第 03 章 Checkpoint

- ☑ 在 OpenAI-compatible 请求中启用 `stream: true` 与 `stream_options.include_usage`；
- ☑ 按 SSE 事件边界解析 `data:` 字段，正确处理 HTTP 分块与 `[DONE]`；
- ☑ 将 SSE 协议转换为 `Start`、`TextDelta`、`Done` 领域事件，经 channel 解耦生产与显示；
- ☑ 每个文本 delta 到达时立即输出，结束后聚合完整回复与 usage；
- ☑ 为流式请求体、分块 SSE、事件顺序、usage 与缺失 `[DONE]` 补充离线测试；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 第 04 章 Checkpoint

- ☑ 提取独立 `Agent`，由其管理 Conversation、待处理输入、执行步数与生命周期状态；
- ☑ 在 Agent 内部完成“调用模型 → 消费/转发 `StreamEvent` → 回填 assistant 回复 → 判断是否继续”的循环；
- ☑ 将“无待处理输入”和“达到最大步数”建模为明确、可观察的终止原因，并拒绝 `max_steps = 0`；
- ☑ 使用离线假模型测试事件转发、状态转移、消息历史更新、正常结束与最大步数保护；
- ☑ 保持 SSE 解析在 LLM 客户端，未提前引入 tool calls、JSON Schema、Tool Registry 或工具执行；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 第 05 章 Checkpoint

- ☑ 定义工具声明、工具调用、finish reason，并支持 assistant `tool_calls` 消息及未来 `role: tool` 的最小消息形状；
- ☑ 仅在有工具时发送标准 OpenAI-compatible `tools` 请求字段，保留流式 `stream` 与 `stream_options.include_usage`；
- ☑ 解析非流式和 SSE 流式文本、usage、finish reason 与分块 tool calls，并在客户端完成聚合；
- ☑ Agent 完整回填 assistant 工具调用后显式停止为 `ToolCallsRequested`，不执行工具；
- ☑ 以离线测试覆盖请求、null content、多调用、分块 arguments、消息入账和 Agent 暂停；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 第 06 章 Checkpoint

- ☑ 定义最小 `Tool` 抽象和启动时组装的 `ToolRegistry`，支持注册、重复名称拒绝、查找及稳定导出工具定义；
- ☑ 在工具执行前解析原始 JSON arguments，由工具完成所需字段、类型与必填项的最小校验；
- ☑ Agent 按模型返回顺序串行分发工具调用，先入账 assistant `tool_calls`，再以关联 `tool_call_id` 写入成功或失败的 `role: tool` JSON 结果，并继续模型循环；
- ☑ 使用确定性、完全离线的 `get_weather` 演示工具，并以 FakeModel 覆盖单调用、多调用、错误回填和工具回合中的最大步数保护；
- ☑ 保持 SSE/tool_calls 协议解析在 LLM 客户端；未引入动态库、WASM、目录发现、热加载或 Plugin Manager；第 14 章 Skills、第 20 章 Plugins 和第 24 章 MCP 再探索动态扩展；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 第 07 章 Checkpoint

- ☑ 仅对 `LlmError::is_retryable()` 认可的网络、`408`、`429` 与 `5xx` 错误进行有限、确定性指数退避重试；普通 `4xx`、配置和协议/JSON 错误不重试；
- ☑ 将 retry/cancel 放在 Agent 的模型调用边界；每次重试重新调用模型，只有完整成功的 `ChatResponse` 才入账 assistant/tool 后续流程；
- ☑ 提供应用内 `CancellationToken`，在模型等待、流事件转发、退避、下一次模型调用与每项同步工具开始前协作式响应取消；已开始同步工具不抢占；
- ☑ 以 `TerminationReason::Cancelled`、`RunReport` 的成功回合/实际尝试/重试计数及最小 `Retrying` / `Cancelled` 事件提供可观察性；
- ☑ 使用完全离线测试覆盖可重试与不可重试错误、耗尽、退避取消、流中取消、工具序列取消，以及第 06 章工具与步数行为不回归；
- ☑ 未实现全局限流、熔断器、任务队列、后台调度、完整 Ctrl+C signal handler、外部进程强杀或复杂 lifecycle event bus；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 第 08 章 Checkpoint

- ☑ 定义最小 `AgentEvent` 生命周期输出，覆盖 started、input、attempt、retry、assistant recorded、tool start/finish、cancelled、finished 与 failed；
- ☑ 将 `Agent::run` 与 `run_with_cancellation` 的输出统一改为 `mpsc::Sender<AgentEvent>`，使显示层只订阅 Agent 边界；
- ☑ 以 `AgentEvent::Model(StreamEvent)` 转发 LLM 流，保留 `StreamEvent::Start`、`TextDelta` 与 `Done` 的模型协议语义，不将 Agent retry/cancel 混入其中；
- ☑ 更新入口与离线 Agent 测试以消费新的事件边界；
- ☑ 未引入全局 event bus、持久化事件日志、订阅过滤、跨进程传递或 observability 后端；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 第 09 章 Checkpoint

- ☑ 新增最小 `ContextManager`，在每次模型调用边界按可配置的消息数量预算生成请求历史，不修改完整会话账本；
- ☑ system 消息始终保留；assistant `tool_calls` 与紧随的 tool 结果只能整体保留或移除，避免向模型发送无对应调用的 tool 结果；
- ☑ 默认不限制历史；可用 `Agent::with_history_message_budget` 配置预算，并用 `AgentEvent::ContextTrimmed` 公开每次请求裁剪的消息数；
- ☑ 不实现摘要、额外模型调用、token 精确计数、向量数据库或长期记忆；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 第 10 章 Checkpoint

- ☑ 使用版本化、确定性 JSON 持久化完整消息账本、待处理输入、可恢复的步数/重试/上下文预算配置及已完成运行报告；
- ☑ `Session::load` 在恢复前拒绝未知格式版本，`Session::restore` 以注入的模型重建 Agent；本地 Tool Registry 保持进程级，由宿主重新附加；
- ☑ 离线测试覆盖 JSON 的确定性写入、待处理输入与历史恢复、未知版本拒绝；
- ☑ 不持久化 API key、模型客户端、取消令牌、事件 channel 或动态工具实例；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 11 章 Checkpoint

- ☑ 新增以 canonical workspace root 约束的 `Workspace`，提供稳定排序的列目录、UTF-8 读文件、创建新文件与显式覆盖已有文件；
- ☑ 拒绝绝对路径和 `..` traversal，并在读/写时验证 canonical 目标或父目录仍在工作区中；
- ☑ 注册 `list_files`、`read_file`、`create_file`、`overwrite_file` 本地工具；创建不会覆盖，删除/重命名不在范围；
- ☑ 离线测试覆盖受限读写、traversal 拒绝和显式覆盖；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 12 章 Checkpoint

- ☑ 新增 workspace-bound `run_inspection`，只允许 `cargo_check`、`cargo_test`、`git_status`、`git_diff`、`git_log` 五种固定检查；
- ☑ 不经 shell 解析用户字符串；Cargo 使用 `--offline`，Git 只允许只读检查，拒绝任意命令、网络与破坏性子命令；
- ☑ 离线测试覆盖 allowlist 解析与任意命令拒绝；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 13 章 Checkpoint

- ☑ 新增无额外依赖的 CLI/slash command 解析：`/help`、`/status`、`/exit`/`/quit`，以及将普通位置参数合并为 Agent prompt；
- ☑ UI-only 命令在加载 LLM 配置前结束，因此不会要求凭据或触发网络；未知 slash command 明确拒绝；
- ☑ 离线测试覆盖命令、prompt 合并与未知命令；手动验证 `/help` 和 `/status`；
- ☑ 不实现全屏 TUI、line editor、历史、颜色、后台输入、多会话选择或复杂命令权限；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 14 章 Checkpoint

- ☑ 新增受 canonical root 约束的 `SkillDirectory`，只列出并按名称按需读取 UTF-8 Markdown 定义；
- ☑ 拒绝 traversal、非 Markdown 文件和不以 Markdown 标题开始的定义；不执行技能内容、不扫描工作区或动态加载代码；
- ☑ 离线测试覆盖稳定列表、按需加载和无效路径/定义；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 15 章 Checkpoint

- ☑ 新增最小 `Subagent`/`ChildAgent` wrapper，只向 child 传入显式 task 和复制的 context；
- ☑ 使用确定性 test double 验证委托边界；不共享 parent 状态、不引入并发框架或调度；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 16 章 Checkpoint

- ☑ `MemoryStore` 从 root-confined Markdown files 按调用者选择的层级顺序注入 context；
- ☑ 拒绝 unsafe scope；不含 DB、embedding、自动检索或自动写入；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 17 章 Checkpoint

- ☑ `ArtifactStore` 用 create-new 将 bytes 外置，并返回 name/size 引用；
- ☑ 拒绝 unsafe names；不含 DB、索引、GC 或远程存储；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 18 章 Checkpoint

- ☑ `HookedTool` 在正常 `Tool` 执行前后调用 `ToolHook`；
- ☑ before 可拒绝，after 可观察结果；不含脚本、全局 event bus 或异步框架；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

## 第 19 章 Checkpoint

- ☑ `MiddlewareModel` 实现既有 `StreamChatModel`，在每次调用前执行 `ModelMiddleware`；
- ☑ 测试覆盖真实 wrapper 边界；不含 middleware 链、洋葱调度或全局 Agent 重写；
- ☑ `cargo fmt --check`、`cargo check`、`cargo clippy -- -D warnings`、`cargo test` 全部通过。

## 当前下一步

进入 **第 20 章 · Plugins**。
