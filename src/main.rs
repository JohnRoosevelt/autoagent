mod llm;
mod message;

use llm::client::LlmClient;
use message::Conversation;

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
        let response = client.chat(conversation.messages()).await?;
        println!("-- 第 {round} 轮 --");
        println!("input_tokens = {}", response.usage.input_tokens);
        println!("output_tokens = {}", response.usage.output_tokens);
        println!("回答: {}", response.content);

        // 将模型回复和下一次追问追加到账本，下一轮才会“记得”这段对话。
        conversation.add_assistant(&response.content);
        conversation.add_user("我刚才说我叫什么？重复我的名字。");
    }
    Ok(())
}
