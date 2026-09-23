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
| ☐ | 05 | Tool / Function Calling | 让模型按 JSON Schema 请求调用宿主程序提供的函数。 |
| ☐ | 06 | Tool Registry | 建立工具注册表、参数校验与工具分发器。 |
| ☐ | 07 | Retry / Cancel | 处理超时、限流、重试，以及 Ctrl+C 取消和资源清理。 |
| ☐ | 08 | Event / Lifecycle | 用事件描述 Agent 发生了什么，让核心逻辑与显示层解耦。 |

## 第二幕 · 让它活着（09–13）

目标：让 Agent 能够管理长上下文、恢复会话，并在代码工作区中完成实际任务。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☐ | 09 | Context Manager | 在上下文接近上限时，按规则裁剪、压缩或保留关键内容。 |
| ☐ | 10 | Session / Resume | 将历史、状态和执行记录持久化；进程重启后可以恢复会话。 |
| ☐ | 11 | Filesystem 工具 | 安全地列出、读取、创建和修改工作区文件。 |
| ☐ | 12 | Shell / Git 工具 | 运行受控命令、查看测试结果与 Git 状态，为 coding agent 提供手和脚。 |
| ☐ | 13 | Command / TUI | 增加斜杠命令与终端交互界面。 |

## 第三幕 · 生长（14–20）

目标：让 Agent 具备可扩展、可组合、可定制的能力。

| 状态 | 章节 | 主题 | 本章目标 |
|---|---:|---|---|
| ☐ | 14 | Skills | 用按需加载的 Markdown 说明目录沉淀任务知识和工作流程。 |
| ☐ | 15 | Subagent | 将具有独立上下文和任务边界的 Agent 包装成工具。 |
| ☐ | 16 | Memory | 建立分级的长期记忆文件，并在合适时注入上下文。 |
| ☐ | 17 | Artifact | 将大对象外置到磁盘或存储中，上下文只保留摘要与引用。 |
| ☐ | 18 | Hooks | 在工具调用前后执行用户定义的回调。 |
| ☐ | 19 | Middleware | 用洋葱式中间件包装模型调用或 Agent 执行流程。 |
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

## 当前下一步

进入 **第 05 章 · Tool / Function Calling**，让模型表达工具请求，并在 Agent 回填 assistant 回复后增加“执行工具 → 写入 tool result → 继续”的分支。
