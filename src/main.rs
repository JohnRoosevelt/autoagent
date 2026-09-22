mod llm;
mod message;

use llm::{StreamEvent, client::LlmClient};
use message::Conversation;
use std::io::Write;
use tokio::sync::mpsc;

// 属性宏：把 async main 改写成同步 main，并在内部构建/启动 tokio 运行时
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // LlmClient 返回可分类的 LlmError；应用入口用 anyhow 统一向用户报告错误。
    let client = LlmClient::from_env()?;
    println!("model = {}", client.model);

    // 对话账本由客户端持有；模型每轮只能看到这次请求中重放的完整历史。
    let mut conversation = Conversation::new();
    conversation.add_system("你是一个简洁、准确的助手。");
    conversation.add_user("我叫小赤。");

    for round in 1..=3 {
        println!("-- 第 {round} 轮 --");
        let (tx, mut rx) = mpsc::channel(32);
        let producer = tokio::spawn({
            let client = client.clone();
            let history = conversation.messages().to_vec();
            async move { client.chat_stream(&history, tx).await }
        });

        // 网络任务生产事件；这里的消费者只负责呈现，不参与 SSE 协议解析。
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
        let response = producer.await??;

        // 将模型回复和下一次追问追加到账本，下一轮才会“记得”这段对话。
        conversation.add_assistant(&response.content);
        conversation.add_user("我刚才说我叫什么？重复我的名字。");
    }
    Ok(())
}
