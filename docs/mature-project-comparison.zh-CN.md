# 第 39 章：与成熟项目的比较

本文是设计比较，并非功能评分卡，也不宣称 AutoAgent 在生产能力上等同于下列任何项目。**已文档化的重点**列中的陈述仅限于截至 2026-09-24 访问到的、已链接的第一手文档。**对 AutoAgent 的启示**列是明确的架构推论，并非对其他项目内部实现的断言。

## 基线

AutoAgent 当前拥有刻意保持精简的核心：宿主通过 `AgentBuilder` 组合 `App`；`Agent` 拥有有界的工具调用循环和完整会话账本；`StreamChatModel` 隔离模型传输；工具被编译进本地注册表；`AgentEvent` 是生命周期事件流。其周边模块大多是可选的本地契约。详细边界图见 [architecture.md](architecture.md)。

## 比较

| 设计 / 产品 | 已文档化的重点 | 相关的 AutoAgent 边界 | 对 AutoAgent 的启示 |
|---|---|---|---|
| [Rig](https://docs.rs/rig-core/latest/rig_core/) | 面向符合人体工程学、模块化 LLM 应用的 Rust 库。其文档描述了提供商无关的补全/嵌入契约、可移植工具、会话记忆契约、向量存储接口，以及单独的经典 agent 运行时。 | `StreamChatModel`、`Tool`、`LocalRetriever`、会话/上下文扩展点。 | 保持提供商、工具、记忆和检索契约能够被独立替换。AutoAgent 已有狭窄的模型/工具边界，但其本地检索器和会话机制应保持可选，而非静默耦合进循环。 |
| [LangGraph](https://docs.langchain.com/oss/python/langgraph/overview) | 面向长时间运行、有状态 agent 的低层编排/运行时；概览强调混合确定性步骤与 LLM 驱动步骤的图、持久化、流式处理和人在回路。 | `Agent` 状态机、`SessionActor`、`SteeringInbox`、调度器/会话状态。 | 单一线性循环适合当前项目。如果工作流需要分支、可持久化的恢复或操作员干预，应显式建模状态转换，而不是给 `Agent` 增加临时标志。 |
| [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/) | 一个更高层的 SDK，提供 agents、工具、交接（handoffs）、护栏、会话、追踪、评估和托管运行循环。 | `Agent`、子 agent 包装器、护栏、可观测性、评估、会话。 | 它展示了小型概念词汇如何组合出更丰富的运行时。除非 AutoAgent 有意承担托管交接、持久化会话和运营追踪的责任，否则应保持当前显式的宿主所有权。 |
| [OpenCode](https://opencode.ai/docs/) | 可在终端、桌面和 IDE 中使用的开源 AI 编码 agent；其简介记载了项目初始化、提供商配置、规划/构建模式以及撤销/重做。 | `command.rs`、工作区工具、未来的展示/传输适配器。 | 编码 agent 的 UX 需要显式的审查/规划边界和变更恢复，而不仅是更多工具。这些是产品工作流要求；它们不会从 AutoAgent 当前的本地工具注册表中自动产生。 |
| [Codex](https://developers.openai.com/codex/) | OpenAI 的文档导航将 Codex 覆盖到 CLI、IDE 扩展、云端、本地环境、工作树、权限、沙箱、技能、MCP 和钩子。 | 宿主组合、权限、沙箱声明、技能/MCP/钩子、`SessionActor`。 | 可用的编码 agent 会分离执行表面、环境、权限和扩展点。AutoAgent 已有这些关注点的早期扩展点，但没有传输层、沙箱执行器或工作树生命周期；文档不得暗示并非如此。 |
| [Claude Code](https://docs.anthropic.com/en/docs/claude-code/overview) | 面向终端、IDE、桌面和浏览器的 agentic 编码工具；其概览记载了代码库编辑、命令执行、MCP、指令/技能/钩子、并行 agents，以及远程或计划工作流。 | 工作区/检查工具、技能、钩子、子 agent、并行执行、服务边界。 | 从产品角度看，能力控制和用户可见的审查与 agent 循环同等重要。AutoAgent 保守的串行默认值和根目录约束是可靠基础，但它缺少其中所述的产品/运行时层。 |
| [Pi](https://github.com/earendil-works/pi) | 该项目将自身描述为一个编码 agent CLI，具备工具调用和状态管理的 agent 运行时、统一的多提供商 LLM API、遥测契约以及单独的持久运行时。其 README 还说明，Pi 不包含内建权限来限制文件系统、进程、网络或凭据访问。 | 模型抽象、`Agent`、可观测性、会话/持久性、`PermissionPolicy`。 | 权限行为必须显而易见并由宿主拥有。AutoAgent 的默认拒绝策略是有益的对照，但 CLI 的显式广泛批准意味着决定实际安全态势的是嵌入宿主，而非仅仅策略类型。 |

## 应保持稳定的内容

比较结果支持保留这些边界，而不是以一个包罗万象的框架层替换它们：

1. **宿主拥有的权限：**凭据、工作区根目录、能力授予以及任何未来传输认证都属于 `Agent` 之外。
2. **显式的编排状态：**`Agent` 仍是当前有界顺序循环的合适归属。仅当存在具体的分支或持久工作流需求时，才应引入图/运行时。
3. **可替换的协议扩展点：**提供商调用保持在 `StreamChatModel` 之后；本地操作保持在 `Tool` 之后；生命周期报告保持在 `AgentEvent` 上。
4. **可选的支撑能力：**检索、记忆、护栏、评估、追踪和服务适配器绝不能隐式改变模型输入或权限。
5. **诚实的产品边界：**UI、多会话管理、远程执行、沙箱强制、持久化任务、批准和运营后端是未来的产品工作，而不是当前本地骨架提供的功能。

## 来源与维护

- Rig core crate 文档：<https://docs.rs/rig-core/latest/rig_core/>
- LangGraph 概览：<https://docs.langchain.com/oss/python/langgraph/overview>
- OpenAI Agents SDK 概览：<https://openai.github.io/openai-agents-python/>
- OpenCode 简介：<https://opencode.ai/docs/>
- Codex 开发者文档：<https://developers.openai.com/codex/>
- Claude Code 概览：<https://docs.anthropic.com/en/docs/claude-code/overview>
- Pi 仓库 README：<https://github.com/earendil-works/pi>

这些产品会独立变化。在使用本文做出集成或安全决策之前，请重新查阅链接的第一手来源以及相关的带版本 API/安全文档。本比较有意不推断许可证、定价、实现架构、安全保证，或超出所列来源陈述范围的功能对等性。
