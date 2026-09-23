# AutoAgent

一个用 Rust 从零实现的 Agent 学习项目。

本项目参考相关 Agent 教程的学习思路，但不以复刻文章代码为目标；会根据自己的理解逐步实现 LLM 调用、错误处理、流式输出、工具调用与 Agent 工作流等能力。

> 当前仍处于早期阶段：已经完成带上下文的 LLM 对话调用、配置加载、SSE 流式输出与最小 Agent Loop。

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
- 通过 SSE 接收 OpenAI-compatible 服务的增量事件，并用 Tokio channel 将 `Start`、文本增量与完成事件交给显示层；
- 由 `Agent` 管理会话、待处理输入与执行步数，在内部循环调用模型、转发流事件并回填 assistant 回复；
- 以“没有待处理输入”或“达到最大步数”明确结束 Agent 运行，防止未来工具分支出现无限循环；
- 提供 `scripts/run.sh`，避免每次手动输入环境变量。

## 项目结构

```text
.
├── src/
│   ├── main.rs          # 程序入口：配置演示输入并显示 Agent 转发的流事件
│   ├── agent.rs         # Agent Loop：会话状态、步数保护、流事件转发与回复回填
│   ├── message.rs       # Role、Message 与 Conversation 对话账本
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

当前入口会创建一段包含 system 消息的会话，并向 `Agent` 排入三条用户输入：

```text
system: 你是一个简洁、准确的助手。
user: 我叫小赤。
```

`Agent` 在每一步才将下一条用户输入写入账本，并把完整历史以 SSE 请求发送给模型。客户端把协议细节转换为 `Start`、`TextDelta`、`Done` 事件；Agent 消费后经 Tokio channel 原样转发给入口显示。收到完成事件后会显示服务端返回的 `input_tokens` 与 `output_tokens`，Agent 再将聚合后的 assistant 回复写回账本。所有输入完成或达到最大步数后，循环明确结束；随着历史变长，通常可以观察到 `input_tokens` 逐轮增加。

## 错误处理约定

- `main.rs` 使用 `anyhow::Result` 作为应用入口的统一错误出口；
- `agent` 模块用 `AgentError` 区分配置、模型、模型任务与事件接收方错误；
- `llm` 模块使用 `thiserror` 定义 `LlmError`，让调用方可以依据错误类型决定是否重试；
- `408`、`429` 与 `5xx` 被视为可重试的 HTTP 错误；
- 缺失或为空的 `AGENT_API_KEY` 会被识别为配置错误，并在发起网络请求前返回。

## 后续方向

以下是计划逐步探索的方向，具体实现会随着学习和实践调整：

- 重试、退避与限流处理；
- 对话历史与上下文管理；
- Tool Calling / Function Calling；
- 多步骤任务规划与执行；
- 更完善的日志、测试与可观测性。

## 开发检查

```sh
cargo fmt --check
cargo check
cargo clippy -- -D warnings
cargo test
```
