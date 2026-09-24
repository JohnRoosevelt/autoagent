# Git Checkpoint 与 Tag 使用说明

AutoAgent 按章节推进。每当一个章节的代码、测试、文档和路线图状态全部完成后，都会创建一个 Git checkpoint：**一个提交 + 一个 annotated tag**。

Tag 用于固定指向“该章节完成时的纯净代码”。后续继续开发不会改变已经发布的章节 checkpoint，因此适合回看、教学和对照路线图。

## 已发布 Checkpoint

| 章节 | Tag | 说明 |
|---:|---|---|
| 01 | `chapter-01-llm-api` | LLM API：OpenRouter 调用、错误分类、响应解析与离线测试。 |
| 02 | `chapter-02-message-context` | Message / Token / Context：对话账本、历史重放、服务端 usage 与三轮 token 演示。 |
| 03 | `chapter-03-streaming` | Streaming / SSE：增量事件解析、统一事件流、实时文本输出与最终 usage 汇总。 |
| 05 | `chapter-05-tool-calling` | Tool / Function Calling：OpenAI-compatible tools/tool_calls 协议映射、流式聚合与 Agent 工具请求暂停状态。 |
| 06 | `chapter-06-tool-registry` | Tool Registry：本地工具注册、最小参数校验、串行分发、`role: tool` 结果回填与继续 Agent Loop。 |
| 07 | `chapter-07-retry-cancel` | Retry / Cancel：有限确定性 retry/backoff 与应用内协作式取消。 |
| 08 | `chapter-08-event-lifecycle` | Event / Lifecycle：以最小 `AgentEvent` 统一 Agent 生命周期与已解析模型流。 |
| 09 | `chapter-09-context-manager` | Context Manager：按消息预算裁剪模型请求并保持工具交换原子性。 |
| 10 | `chapter-10-session-resume` | Session / Resume：版本化确定性 JSON 的会话保存与恢复。 |
| 11 | `chapter-11-filesystem-tools` | Filesystem 工具：受工作区根目录约束的安全文件访问。 |
| 12 | `chapter-12-shell-git-tools` | Shell / Git 工具：离线、allowlist 的只读工作区检查。 |
| 13 | `chapter-13-command-tui` | Command / TUI：最小 CLI/slash command 入口。 |
| 14 | `chapter-14-skills` | Skills：root-confined 的按需 Markdown 定义加载。 |
| 15 | `chapter-15-subagent` | Subagent：显式 task/context 边界的同步 child wrapper。 |
| 16 | `chapter-16-memory` | Memory：root-confined 的显式分级 Markdown 注入。 |
| 17 | `chapter-17-artifact` | Artifact：root-confined 的外置 bytes 与小引用。 |
| 18 | `chapter-18-hooks` | Hooks：Tool pre/post 的本地 callback wrapper。 |
| 19 | `chapter-19-middleware` | Middleware：最小 StreamChatModel call wrapper。 |
| 20 | `chapter-20-plugins` | Plugins：manifest/registry convention，无动态加载。 |
| 21 | `chapter-21-configuration` | Configuration：defaults < file < environment 的非私密设置合并。 |
| 22 | `chapter-22-permission` | Permission：default-deny 的敏感能力审批。 |
| 23 | `chapter-23-sandbox` | Sandbox：声明受限 profile，拒绝 untrusted execution。 |
| 24 | `chapter-24-mcp` | MCP：JSON-RPC protocol 与 local test double。 |
| 25 | `chapter-25-compaction` | Compaction：local summary replacement request view。 |
| 26 | `chapter-26-job-scheduler` | Job / Scheduler：in-memory enqueue/query/cancel state machine。 |
| 27 | `chapter-27-model-router` | Model Router：deterministic primary/fallback candidates。 |

第 04 章完成后的预定 checkpoint 名称为 `chapter-04-agent-loop`；本地 checkpoint 尚未创建前，不应将其列为已发布 Tag。该章固定 Agent Loop 的会话状态、事件转发和最大步数终止保护。

第 06 章的 Registry 仅在程序启动时组装编译进二进制的 Rust 工具；它不是动态库、WASM、目录发现、热加载或 Plugin Manager。更进一步的动态扩展留待第 14 章 Skills、第 20 章 Plugins 与第 24 章 MCP。

第 07 章已发布为 `chapter-07-retry-cancel`，第 08 章已发布为 `chapter-08-event-lifecycle`。第 08 章不包含全局 event bus、持久化事件日志、订阅过滤、跨进程事件传递或 observability 后端。

## 第 09 章 Checkpoint（已发布为 `chapter-09-context-manager`）

- ☑ 在模型调用边界按可配置消息预算裁剪请求历史，账本保持完整；
- ☑ system 消息优先保留，assistant `tool_calls` 与连续 tool 结果作为不可分割单元；
- ☑ 以 `AgentEvent::ContextTrimmed` 暴露每次裁剪数量；
- ☑ 不包含摘要、额外模型调用、token 精确计数或向量数据库。

## 第 10 章 Checkpoint（已发布为 `chapter-10-session-resume`）

- ☑ 持久化会话账本、待处理输入、可恢复配置与执行报告；
- ☑ 用版本化确定性 JSON 往返恢复，拒绝未知版本；
- ☑ 不持久化密钥、模型客户端或本地工具实例。

## 第 11 章 Checkpoint（已发布为 `chapter-11-filesystem-tools`）

- ☑ canonical 工作区约束、绝对路径/`..` traversal 拒绝与已有目标 symlink 检查；
- ☑ 列目录、读文件、创建与显式覆盖；不包含删除或重命名。

## 第 12 章 Checkpoint（已发布为 `chapter-12-shell-git-tools`）

- ☑ 固定 allowlist 的 Cargo/Git 检查，无 shell 或任意命令；
- ☑ Cargo 离线运行，Git 子命令为只读检查；拒绝破坏性与网络命令。

## 第 13 章 Checkpoint（已发布为 `chapter-13-command-tui`）

- ☑ 依赖无关的 `/help`、`/status`、`/exit`/`/quit` 和位置参数 prompt 解析；
- ☑ UI-only 命令在加载 LLM 配置或发起网络前退出；未知 slash command 拒绝；
- ☑ 不包含全屏 TUI、line editor 或交互历史。

## 第 14 章 Checkpoint（已发布为 `chapter-14-skills`）

- ☑ Markdown-only 的 `SkillDirectory` 以 canonical root 限制技能定义；
- ☑ 仅按名称列出和读取，拒绝 traversal、非定义内容与任何代码执行；
- ☑ 不包含目录自动执行、热加载或动态插件。

## 第 15 章 Checkpoint（已发布为 `chapter-15-subagent`）

- ☑ 子 Agent 仅接收显式任务和复制的上下文；
- ☑ 使用确定性 test double；不含并发、调度或 parent state 共享。

## 第 16 章 Checkpoint（已发布为 `chapter-16-memory`）

- ☑ 按调用者选定顺序读取受限根目录内的 Markdown memories；
- ☑ 不使用数据库、embedding 或自动检索。

## 第 17 章 Checkpoint（已发布为 `chapter-17-artifact`）

- ☑ 将 bytes create-new 写入受限目录，仅返回 name/size reference；
- ☑ 不包含 DB、索引、GC 或远程存储。

## 第 18 章 Checkpoint（已发布为 `chapter-18-hooks`）

- ☑ Tool wrapper 调用可拒绝的 pre-hook 和可观察结果的 post-hook；
- ☑ 不包含脚本、全局 event bus 或异步 callback framework。

## 第 19 章 Checkpoint（已发布为 `chapter-19-middleware`）

- ☑ `MiddlewareModel` 包装既有模型调用边界；
- ☑ 不包含 middleware chain、洋葱调度或全局 Agent 重写。

## 第 20 章 Checkpoint（已发布为 `chapter-20-plugins`）

- ☑ serializable manifest 与稳定的 registry convention；
- ☑ 不扫描目录、不动态加载、不使用 dynamic library 或 WASM。

## 第 21 章 Checkpoint（已发布为 `chapter-21-configuration`）

- ☑ 测试化的 defaults < local file < environment 分层设置；
- ☑ 不处理、保存或显示 API key。

## 第 22 章 Checkpoint（已发布为 `chapter-22-permission`）

- ☑ 默认拒绝写入、删除、命令与网络 capability；
- ☑ 仅建模审批，不执行操作。

## 第 23 章 Checkpoint（已发布为 `chapter-23-sandbox`）

- ☑ 声明禁网、只读的 sandbox profile；
- ☑ untrusted execution 一律不可用；无进程或容器启动。

## 第 24 章 Checkpoint（已发布为 `chapter-24-mcp`）

- ☑ MCP JSON-RPC 2.0 initialize protocol 和 manifest；
- ☑ 只使用 local test double；无外部连接、进程或网络。

## 第 25 章 Checkpoint（已发布为 `chapter-25-compaction`）

- ☑ 以 caller-supplied summary 替换旧请求历史；
- ☑ 不调用模型，不改写完整账本。

## 第 26 章 Checkpoint（已发布为 `chapter-26-job-scheduler`）

- ☑ 内存 job 的 enqueue、query、cancel；
- ☑ 不执行、恢复或调度后台任务。

## 第 27 章 Checkpoint（已发布为 `chapter-27-model-router`）

- ☑ 输出 primary 后 fallback 的稳定模型候选顺序；
- ☑ 无 provider 调用或自动重试策略。

## 查看 Tag

列出所有章节 checkpoint：

```sh
git tag --list 'chapter-*'
```

查看某个 Tag 的说明和它指向的提交：

```sh
git show chapter-01-llm-api
```

只查看 Tag 附带的说明：

```sh
git tag -n1 chapter-01-llm-api
```

## 查看某章的纯净代码

切换到第 01 章完成时的代码：

```sh
git checkout chapter-01-llm-api
```

此时 Git 会处于 `detached HEAD` 状态。这不是错误，只表示当前查看的是固定 checkpoint，而不是某个可继续前进的分支。

查看完后，回到持续开发分支：

```sh
git checkout master
```

## 从某个 Checkpoint 开始实践或修复

不要直接在 `detached HEAD` 状态修改并提交。需要基于某章代码继续实验时，先创建分支：

```sh
git checkout -b chapter-01-practice chapter-01-llm-api
```

这样新分支从第 01 章的纯净代码开始，你可以自由修改，而原始 Tag 保持不变。

如果 Git 版本支持 `switch`，等价写法是：

```sh
git switch -c chapter-01-practice chapter-01-llm-api
```

## 每章完成后的发布流程

以第 04 章为例：

```sh
# 1. 确认质量检查通过
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test

# 2. 确认路线图已更新为完成状态
# 3. 提交本章全部相关改动
git add README.md ROADMAP.md src docs
git commit -m "Complete chapter 04: Agent Loop"

# 4. 创建带说明的 annotated tag
git tag -a chapter-04-agent-loop -m "Chapter 04: Agent Loop checkpoint"

# 5. 推送提交与 tag
git push origin master
git push origin chapter-04-agent-loop
```

## 命名约定

统一使用：

```text
chapter-<两位章节号>-<简短主题>
```

示例：

```text
chapter-01-llm-api
chapter-02-message-context
chapter-03-streaming
chapter-04-agent-loop
chapter-05-tool-calling
```

不要移动、删除或复用已经推送的 checkpoint tag。它们应当始终代表某一章完成时的固定代码状态。
