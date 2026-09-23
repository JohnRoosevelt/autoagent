use crate::{
    agent::StreamChatModel,
    llm::{ChatResponse, LlmError, StreamEvent, Usage},
    message::Message,
};
use futures::StreamExt;
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
    /// 保留此非流式路径，方便与 SSE 路径对照或供后续调用方选择。
    #[allow(dead_code)]
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

    /// 以 Server-Sent Events 接收增量回复，并将统一后的领域事件发送给调用方。
    ///
    /// 返回值中的 content 是所有 delta 的拼接结果。请求使用
    /// `stream_options.include_usage`，以便支持该选项的服务在结束事件中返回 token 用量。
    pub async fn chat_stream(
        &self,
        messages: &[Message],
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<ChatResponse, LlmError> {
        let body = stream_request_body(&self.model, messages);
        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp
                .text()
                .await
                .map_err(|e| LlmError::Network(e.to_string()))?;
            return Err(LlmError::from_http(status.as_u16(), text));
        }

        send_stream_event(&tx, StreamEvent::Start).await?;

        let mut parser = SseParser::default();
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| LlmError::Network(e.to_string()))?;
            for delta in parser.push(&chunk)? {
                send_stream_event(&tx, StreamEvent::TextDelta(delta)).await?;
            }
        }

        let (response, final_deltas) = parser.finish()?;
        for delta in final_deltas {
            send_stream_event(&tx, StreamEvent::TextDelta(delta)).await?;
        }
        send_stream_event(&tx, StreamEvent::Done(response.clone())).await?;
        Ok(response)
    }
}

impl StreamChatModel for LlmClient {
    #[allow(clippy::manual_async_fn)]
    fn chat_stream(
        &self,
        messages: &[Message],
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> impl std::future::Future<Output = Result<ChatResponse, LlmError>> + Send {
        async move { LlmClient::chat_stream(self, messages, tx).await }
    }
}

#[allow(dead_code)]
fn chat_request_body(model: &str, messages: &[Message]) -> Value {
    serde_json::json!({
        "model": model,
        "messages": messages,
    })
}

async fn send_stream_event(
    tx: &tokio::sync::mpsc::Sender<StreamEvent>,
    event: StreamEvent,
) -> Result<(), LlmError> {
    tx.send(event)
        .await
        .map_err(|_| LlmError::Other("流事件接收方已关闭".into()))
}

fn stream_request_body(model: &str, messages: &[Message]) -> Value {
    serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
        "stream_options": { "include_usage": true },
    })
}

#[allow(dead_code)]
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

/// 最小 SSE 解码器：HTTP chunk 不等于 SSE 事件，因此先按换行重组，再在空行处分发事件。
#[derive(Default)]
struct SseParser {
    pending: Vec<u8>,
    data: Vec<String>,
    content: String,
    usage: Usage,
    done: bool,
}

impl SseParser {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, LlmError> {
        let mut deltas = Vec::new();
        self.pending.extend_from_slice(bytes);
        while let Some(newline) = self.pending.iter().position(|byte| *byte == b'\n') {
            let mut line = self.pending.drain(..=newline).collect::<Vec<_>>();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let line = std::str::from_utf8(&line)
                .map_err(|e| LlmError::Other(format!("SSE 事件不是 UTF-8: {e}")))?;
            self.process_line(line, &mut deltas)?;
        }
        Ok(deltas)
    }

    fn finish(mut self) -> Result<(ChatResponse, Vec<String>), LlmError> {
        let mut deltas = Vec::new();
        if !self.pending.is_empty() {
            let line = std::str::from_utf8(&self.pending)
                .map_err(|e| LlmError::Other(format!("SSE 事件不是 UTF-8: {e}")))?
                .to_owned();
            self.process_line(&line, &mut deltas)?;
        }
        self.dispatch(&mut deltas)?;
        if !self.done {
            return Err(LlmError::Other("SSE 流在收到 [DONE] 前结束".into()));
        }
        Ok((
            ChatResponse {
                content: self.content,
                usage: self.usage,
            },
            deltas,
        ))
    }

    fn process_line(&mut self, line: &str, deltas: &mut Vec<String>) -> Result<(), LlmError> {
        if line.is_empty() {
            return self.dispatch(deltas);
        }
        if let Some(data) = line.strip_prefix("data:") {
            self.data.push(data.trim_start().to_owned());
        }
        Ok(())
    }

    fn dispatch(&mut self, deltas: &mut Vec<String>) -> Result<(), LlmError> {
        if self.data.is_empty() {
            return Ok(());
        }
        let data = self.data.join("\n");
        self.data.clear();
        if data == "[DONE]" {
            self.done = true;
            return Ok(());
        }

        let value: Value = serde_json::from_str(&data)
            .map_err(|e| LlmError::Other(format!("SSE data 不是合法 JSON: {e}; data: {data}")))?;
        if let Some(content) = value["choices"][0]["delta"]["content"].as_str() {
            self.content.push_str(content);
            deltas.push(content.to_owned());
        }
        if value.get("usage").is_some_and(|usage| !usage.is_null()) {
            self.usage = Usage {
                input_tokens: parse_token_count(&value, "prompt_tokens", &data)?,
                output_tokens: parse_token_count(&value, "completion_tokens", &data)?,
            };
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
