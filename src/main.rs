mod agent;
#[allow(dead_code)]
mod artifact;
mod command;
#[allow(dead_code)]
mod compaction;
#[allow(dead_code)]
mod configuration;
mod context;
#[allow(dead_code)]
mod cost;
#[allow(dead_code)]
mod evaluation;
mod filesystem;
mod framework;
#[allow(dead_code)]
mod guardrails;
#[allow(dead_code)]
mod hooks;
mod llm;
#[allow(dead_code)]
mod mcp;
#[allow(dead_code)]
mod memory;
mod message;
#[allow(dead_code)]
mod middleware;
#[allow(dead_code)]
mod model_router;
#[allow(dead_code)]
mod observability;
#[allow(dead_code)]
mod parallel;
#[allow(dead_code)]
mod permission;
#[allow(dead_code)]
mod plugins;
#[allow(dead_code)]
mod sandbox;
#[allow(dead_code)]
mod scheduler;
#[allow(dead_code)]
mod session;
mod shell;
#[allow(dead_code)]
mod skills;
#[allow(dead_code)]
mod steering;
#[allow(dead_code)]
mod subagent;
mod tool;

use agent::{AgentEvent, RetryPolicy};
use command::Command;
use configuration::AppConfig;
use filesystem::Workspace;
use framework::AgentBuilder;
use llm::{FinishReason, StreamEvent, client::LlmClient};
use message::Role;
use permission::{Capability, PermissionPolicy};
use std::{collections::BTreeMap, io::Write, time::Duration};
use tokio::sync::mpsc;

// 属性宏：把 async main 改写成同步 main，并在内部构建/启动 tokio 运行时
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let prompt = match command::parse_args(std::env::args().skip(1))? {
        Command::Help => {
            println!("{}", command::help_text());
            return Ok(());
        }
        Command::Status => {
            println!(
                "AutoAgent ready. Workspace tools are root-confined; inspections are offline and allowlisted."
            );
            return Ok(());
        }
        Command::Exit => return Ok(()),
        Command::Prompt(prompt) => prompt,
    };
    let environment = std::env::vars().collect::<BTreeMap<_, _>>();
    let config = AppConfig::load(None, &environment)?;
    let client = LlmClient::from_config(&config)?;
    println!("model = {}", client.model);

    // The CLI preserves its existing workspace-tool surface by explicitly approving it.
    let policy = PermissionPolicy::deny_all()
        .allow(Capability::FileWrite)
        .allow(Capability::CommandExecution)
        .allow(Capability::NetworkAccess);
    let workspace = Workspace::new(std::env::current_dir()?)?;
    let mut app = AgentBuilder::new(client, config, workspace)
        .with_policy(policy)
        .with_system_prompt("你是一个简洁、准确的助手。")
        .with_retry_policy(RetryPolicy::new(2, Duration::from_millis(250)))
        .with_history_message_budget(32)
        .build()?;
    app.enqueue_user(prompt);

    let (tx, mut rx) = mpsc::channel(32);
    let renderer = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::Started => println!("Agent 已启动。"),
                AgentEvent::Input { content } => println!("用户输入: {content}"),
                AgentEvent::Attempt { attempt } => println!("开始第 {attempt} 次模型尝试"),
                AgentEvent::Retry { attempt, delay } => {
                    println!("模型请求失败，将在 {:?} 后开始第 {attempt} 次尝试", delay);
                }
                AgentEvent::Model(StreamEvent::Start) => print!("回答: "),
                AgentEvent::Model(StreamEvent::TextDelta(delta)) => {
                    print!("{delta}");
                    std::io::stdout().flush().expect("无法刷新标准输出");
                }
                AgentEvent::Model(StreamEvent::Done(response)) => {
                    println!();
                    println!("input_tokens = {}", response.usage.input_tokens);
                    println!("output_tokens = {}", response.usage.output_tokens);
                    if response.finish_reason == FinishReason::ToolCalls {
                        println!("模型请求工具调用:");
                        for call in &response.tool_calls {
                            println!("  {} {}({})", call.id, call.name, call.arguments);
                        }
                    }
                }
                AgentEvent::AssistantRecorded { .. } => {}
                AgentEvent::ToolStarted { call_id, name } => println!("开始工具 {call_id}: {name}"),
                AgentEvent::ToolFinished { call_id, name } => {
                    println!("完成工具 {call_id}: {name}")
                }
                AgentEvent::ContextTrimmed { removed_messages } => {
                    println!("模型请求前裁剪了 {removed_messages} 条历史消息")
                }
                AgentEvent::Cancelled => println!("Agent 已取消。"),
                AgentEvent::Finished(_) => println!("Agent 已完成。"),
                AgentEvent::Failed { error } => eprintln!("Agent 失败: {error}"),
            }
        }
    });

    let report = app.run(tx).await?;
    renderer.await?;
    for message in app.agent().conversation().messages() {
        if message.role == Role::Tool {
            println!(
                "工具结果 {}: {}",
                message.tool_call_id.as_deref().unwrap_or_default(),
                message.content.as_deref().unwrap_or_default()
            );
        }
    }
    println!(
        "Agent 结束: {:?}，成功模型回合 {}，实际尝试 {}，重试 {}，已处理 {} 个工具调用",
        report.termination,
        report.steps_taken,
        report.model_attempts,
        report.retries,
        report.tool_calls_processed
    );
    Ok(())
}
