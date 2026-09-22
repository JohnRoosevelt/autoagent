use crate::{
    llm::{ChatResponse, LlmError, Usage},
    message::Message,
};
use serde_json::Value;

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

    /// 将完整对话历史发送给模型；调用方负责在每轮结束后把回复追加回会话。
    // async fn：异步函数，调用后返回一个 Future，要 .await 才真正执行到完成
    pub async fn chat(&self, messages: &[Message]) -> Result<ChatResponse, LlmError> {
        let body = chat_request_body(&self.model, messages);

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

        parse_chat_response(&text)
    }
}

fn chat_request_body(model: &str, messages: &[Message]) -> Value {
    serde_json::json!({
        "model": model,
        "messages": messages,
    })
}

fn parse_chat_response(text: &str) -> Result<ChatResponse, LlmError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|e| LlmError::Other(format!("响应不是合法 JSON: {e}")))?;

    let content = value["choices"][0]["message"]["content"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| LlmError::Other(format!("响应结构不符合预期: {text}")))?;

    let usage = Usage {
        input_tokens: parse_token_count(&value, "prompt_tokens", text)?,
        output_tokens: parse_token_count(&value, "completion_tokens", text)?,
    };

    Ok(ChatResponse { content, usage })
}

fn parse_token_count(value: &Value, field: &str, text: &str) -> Result<usize, LlmError> {
    value["usage"][field]
        .as_u64()
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| LlmError::Other(format!("响应缺少 usage.{field}: {text}")))
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
