mod agent;
mod llm;
mod message;

use agent::Agent;
use llm::{StreamEvent, client::LlmClient};
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
    let mut agent = Agent::new(client, conversation, 3)?;
    agent.enqueue_user("我叫小赤。");
    agent.enqueue_user("我刚才说我叫什么？重复我的名字。");
    agent.enqueue_user("请再次确认我的名字。");

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
    Ok(())
}
