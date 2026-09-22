mod llm;

use llm::client::LlmClient;

// 属性宏：把 async main 改写成同步 main，并在内部构建/启动 tokio 运行时
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // LlmClient 返回可分类的 LlmError；应用入口用 anyhow 统一向用户报告错误。
    let client = LlmClient::from_env()?;
    println!("model = {}", client.model);

    let answer = client.chat_raw("用一句话解释什么是 HTTP").await?;
    println!("{answer}");
    Ok(())
}
