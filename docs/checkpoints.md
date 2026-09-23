# Git Checkpoint 与 Tag 使用说明

AutoAgent 按章节推进。每当一个章节的代码、测试、文档和路线图状态全部完成后，都会创建一个 Git checkpoint：**一个提交 + 一个 annotated tag**。

Tag 用于固定指向“该章节完成时的纯净代码”。后续继续开发不会改变已经发布的章节 checkpoint，因此适合回看、教学和对照路线图。

## 已发布 Checkpoint

| 章节 | Tag | 说明 |
|---:|---|---|
| 01 | `chapter-01-llm-api` | LLM API：OpenRouter 调用、错误分类、响应解析与离线测试。 |
| 02 | `chapter-02-message-context` | Message / Token / Context：对话账本、历史重放、服务端 usage 与三轮 token 演示。 |
| 03 | `chapter-03-streaming` | Streaming / SSE：增量事件解析、统一事件流、实时文本输出与最终 usage 汇总。 |

第 04 章完成后的预定 checkpoint 名称为 `chapter-04-agent-loop`；本地 checkpoint 尚未创建前，不应将其列为已发布 Tag。该章固定 Agent Loop 的会话状态、事件转发和最大步数终止保护。

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
