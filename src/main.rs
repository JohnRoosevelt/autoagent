mod agent;
mod llm;
mod message;

use agent::Agent;
use llm::{FinishReason, StreamEvent, ToolDefinition, client::LlmClient};
use message::Conversation;
use std::io::Write;
use tokio::sync::mpsc;

// 属性宏：把 async main 改写成同步 main，并在内部构建/启动 tokio 运行时
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = LlmClient::from_env()?;
    println!("model = {}", client.model);

    let mut conversation = Conversation::new();
    conversation.add_system("你是一个简洁、准确的助手。");
    let weather_tool = ToolDefinition::new(
        "get_weather",
        "查询指定城市的当前天气。",
        serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string", "description": "城市名称" } },
            "required": ["city"],
            "additionalProperties": false
        }),
    );
    let mut agent = Agent::new(client, conversation, 3)?.with_tools(vec![weather_tool]);
    agent.enqueue_user("请查询北京现在的天气。请调用 get_weather，不要猜测结果。");

    let (tx, mut rx) = mpsc::channel(32);
    let renderer = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::Start => print!("回答: "),
                StreamEvent::TextDelta(delta) => {
                    print!("{delta}");
                    std::io::stdout().flush().expect("无法刷新标准输出");
                }
                StreamEvent::Done(response) => {
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
            }
        }
    });

    let report = agent.run(tx).await?;
    renderer.await?;
    println!(
        "Agent 结束: {:?}，共 {} 步",
        report.termination, report.steps_taken
    );
    if !report.requested_tool_calls.is_empty() {
        println!("本章不会执行以上工具；第 06 章会在此处追加 role: tool 结果并继续循环。");
    }
    Ok(())
}
