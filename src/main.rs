mod agent;
mod context;
mod filesystem;
mod llm;
mod message;
#[allow(dead_code)]
mod session;
mod tool;

use agent::{Agent, AgentEvent, RetryPolicy};
use filesystem::Workspace;
use llm::{FinishReason, StreamEvent, client::LlmClient};
use message::{Conversation, Role};
use std::{io::Write, time::Duration};
use tokio::sync::mpsc;
use tool::{
    CreateFileTool, GetWeatherTool, ListFilesTool, OverwriteFileTool, ReadFileTool, ToolRegistry,
};

// 属性宏：把 async main 改写成同步 main，并在内部构建/启动 tokio 运行时
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = LlmClient::from_env()?;
    println!("model = {}", client.model);

    let mut conversation = Conversation::new();
    conversation.add_system("你是一个简洁、准确的助手。");
    let mut tools = ToolRegistry::new();
    tools.register(GetWeatherTool)?;
    let workspace = Workspace::new(std::env::current_dir()?)?;
    tools.register(ListFilesTool::new(workspace.clone()))?;
    tools.register(ReadFileTool::new(workspace.clone()))?;
    tools.register(CreateFileTool::new(workspace.clone()))?;
    tools.register(OverwriteFileTool::new(workspace))?;
    let mut agent = Agent::new(client, conversation, 3)?
        .with_tool_registry(tools)
        .with_retry_policy(RetryPolicy::new(2, Duration::from_millis(250)))
        .with_history_message_budget(32);
    agent.enqueue_user("请查询北京现在的天气。请调用 get_weather，不要猜测结果。");

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

    let report = agent.run(tx).await?;
    renderer.await?;
    for message in agent.conversation().messages() {
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
