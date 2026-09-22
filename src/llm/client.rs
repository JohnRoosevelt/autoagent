use crate::llm::LlmError;
use serde_json::{Value, json};

// 派生宏：为结构体自动生成 Clone 实现（对本类型来说 clone 很廉价，见下面 http 字段的说明）
#[derive(Clone)]
pub struct LlmClient {
    /// reqwest::Client 内部自带连接池（持久连接 + TLS 会话复用），
    /// clone 它只是引用计数 +1，非常便宜。
    /// 千万不要每次请求 new 一个——那等于每次都重新 TCP/TLS 握手。
    http: reqwest::Client,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl LlmClient {
    /// 三个环境变量决定"我们用的是哪家模型"：
    /// AGENT_BASE_URL / AGENT_API_KEY / AGENT_MODEL
    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("AGENT_API_KEY")
            .map_err(|_| LlmError::Config("缺少 AGENT_API_KEY，请在 .env 中设置它".into()))?;

        if api_key.trim().is_empty() {
            return Err(LlmError::Config("AGENT_API_KEY 不能为空".into()));
        }

        let base_url = std::env::var("AGENT_BASE_URL")
            .unwrap_or_else(|_| "https://openrouter.ai/api/v1".into());

        Ok(Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_owned(),
            api_key,
            model: std::env::var("AGENT_MODEL").unwrap_or_else(|_| "openrouter/free".into()),
        })
    }

    /// 最朴素也最诚实的版本：手拼请求 JSON，POST，按固定路径取结果。
    // async fn：异步函数，调用后返回一个 Future，要 .await 才真正执行到完成
    pub async fn chat_raw(&self, user_message: &str) -> Result<String, LlmError> {
        // json! 宏：用类 JSON 字面量直接构造 serde_json::Value，位置上可插任意表达式
        let body = json!({
          "model": self.model,
          "messages": [
            { "role": "user", "content": user_message }
          ]
        });

        // .await 等待异步操作完成；? 在 Err 时提前返回，并经 From 把错误转换成 anyhow::Error
        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;

        let status = resp.status();
        // 先收成字符串再解析：失败时 body 里的报错信息不会丢
        let text = resp
            .text()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;

        if !status.is_success() {
            // 服务器返回非 2xx 状态码，解析 body 中的错误信息
            return Err(LlmError::from_http(status.as_u16(), text));
        }

        // ? 依赖 From 自动转换：serde_json 的解析错误在这里被转成 LlmError::Other 再向上传播
        let value: Value = serde_json::from_str(&text)
            .map_err(|e| LlmError::Other(format!("响应不是合法 JSON: {e}")))?;
        // choices[0].message.content —— OpenAI 协议的固定取值路径
        // Value 实现了 Index：像 JS 一样链式索引，取不到的键返回 Value::Null 而不 panic
        // as_str 把 Value 转成 Option<&str>；ok_or_else 把 Option 变 Result（None 时才执行
        // 闭包构造错误）；{text} 是 format! 的内联捕获，等价于 "…{}", text
        value["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| LlmError::Other(format!("响应结构不符合预期: {text}")))
    }
}
