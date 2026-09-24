use crate::{
    agent::StreamChatModel,
    llm::{ChatResponse, FinishReason, LlmError, StreamEvent, ToolCall, ToolDefinition, Usage},
    message::Message,
};
use futures::StreamExt;
use serde_json::Value;

#[derive(Clone)]
pub struct LlmClient {
    http: reqwest::Client,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl LlmClient {
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

    #[allow(dead_code)]
    pub async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
    ) -> Result<ChatResponse, LlmError> {
        let body = chat_request_body(&self.model, messages, tools);
        let resp = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;
        if !status.is_success() {
            return Err(LlmError::from_http(status.as_u16(), text));
        }
        parse_chat_response(&text)
    }

    pub async fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<ChatResponse, LlmError> {
        let body = stream_request_body(&self.model, messages, tools);
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
            for delta in parser.push(&chunk.map_err(|e| LlmError::Network(e.to_string()))?)? {
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
        tools: &[ToolDefinition],
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> impl std::future::Future<Output = Result<ChatResponse, LlmError>> + Send {
        async move { LlmClient::chat_stream(self, messages, tools, tx).await }
    }
}

#[allow(dead_code)]
fn chat_request_body(model: &str, messages: &[Message], tools: &[ToolDefinition]) -> Value {
    let mut body = serde_json::json!({ "model": model, "messages": messages });
    if !tools.is_empty() {
        body["tools"] = tool_definitions_value(tools);
    }
    body
}

fn stream_request_body(model: &str, messages: &[Message], tools: &[ToolDefinition]) -> Value {
    let mut body = serde_json::json!({ "model": model, "messages": messages, "stream": true, "stream_options": { "include_usage": true } });
    if !tools.is_empty() {
        body["tools"] = tool_definitions_value(tools);
    }
    body
}

fn tool_definitions_value(tools: &[ToolDefinition]) -> Value {
    Value::Array(tools.iter().map(|tool| serde_json::json!({ "type": "function", "function": { "name": tool.name, "description": tool.description, "parameters": tool.parameters } })).collect())
}

async fn send_stream_event(
    tx: &tokio::sync::mpsc::Sender<StreamEvent>,
    event: StreamEvent,
) -> Result<(), LlmError> {
    tx.send(event)
        .await
        .map_err(|_| LlmError::Other("流事件接收方已关闭".into()))
}

#[allow(dead_code)]
fn parse_chat_response(text: &str) -> Result<ChatResponse, LlmError> {
    let value: Value = serde_json::from_str(text)
        .map_err(|e| LlmError::Other(format!("响应不是合法 JSON: {e}")))?;
    let choice = &value["choices"][0];
    let message = &choice["message"];
    if !message.is_object() {
        return Err(LlmError::Other(format!("响应结构不符合预期: {text}")));
    }
    let content = message["content"].as_str().unwrap_or_default().to_owned();
    let tool_calls = parse_tool_calls(&message["tool_calls"], text)?;
    let finish_reason = parse_finish_reason(choice["finish_reason"].as_str());
    let usage = Usage {
        input_tokens: parse_token_count(&value, "prompt_tokens", text)?,
        output_tokens: parse_token_count(&value, "completion_tokens", text)?,
    };
    Ok(ChatResponse {
        content,
        tool_calls,
        finish_reason,
        usage,
    })
}

fn parse_tool_calls(value: &Value, text: &str) -> Result<Vec<ToolCall>, LlmError> {
    let Some(calls) = value.as_array() else {
        return if value.is_null() {
            Ok(Vec::new())
        } else {
            Err(LlmError::Other(format!(
                "响应 tool_calls 不符合预期: {text}"
            )))
        };
    };
    calls
        .iter()
        .map(|call| {
            Ok(ToolCall {
                id: call["id"]
                    .as_str()
                    .ok_or_else(|| LlmError::Other(format!("响应 tool_call 缺少 id: {text}")))?
                    .to_owned(),
                name: call["function"]["name"]
                    .as_str()
                    .ok_or_else(|| {
                        LlmError::Other(format!("响应 tool_call 缺少 function.name: {text}"))
                    })?
                    .to_owned(),
                arguments: call["function"]["arguments"]
                    .as_str()
                    .ok_or_else(|| {
                        LlmError::Other(format!("响应 tool_call 缺少 function.arguments: {text}"))
                    })?
                    .to_owned(),
            })
        })
        .collect()
}

fn parse_finish_reason(reason: Option<&str>) -> FinishReason {
    match reason {
        Some("stop") => FinishReason::Stop,
        Some("tool_calls") => FinishReason::ToolCalls,
        Some(other) => FinishReason::Other(other.to_owned()),
        None => FinishReason::Other("missing".into()),
    }
}
fn parse_token_count(value: &Value, field: &str, text: &str) -> Result<usize, LlmError> {
    value["usage"][field]
        .as_u64()
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| LlmError::Other(format!("响应缺少 usage.{field}: {text}")))
}

#[derive(Default)]
struct SseParser {
    pending: Vec<u8>,
    data: Vec<String>,
    content: String,
    tool_calls: Vec<PartialToolCall>,
    finish_reason: FinishReason,
    usage: Usage,
    done: bool,
}
#[derive(Default)]
struct PartialToolCall {
    id: String,
    name: String,
    arguments: String,
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
        let tool_calls = self
            .tool_calls
            .into_iter()
            .map(|call| ToolCall {
                id: call.id,
                name: call.name,
                arguments: call.arguments,
            })
            .collect();
        Ok((
            ChatResponse {
                content: self.content,
                tool_calls,
                finish_reason: self.finish_reason,
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
        let choice = &value["choices"][0];
        let delta = &choice["delta"];
        if let Some(content) = delta["content"].as_str() {
            self.content.push_str(content);
            deltas.push(content.to_owned());
        }
        if let Some(reason) = choice["finish_reason"].as_str() {
            self.finish_reason = parse_finish_reason(Some(reason));
        }
        self.append_tool_call_deltas(&delta["tool_calls"])?;
        if value.get("usage").is_some_and(|usage| !usage.is_null()) {
            self.usage = Usage {
                input_tokens: parse_token_count(&value, "prompt_tokens", &data)?,
                output_tokens: parse_token_count(&value, "completion_tokens", &data)?,
            };
        }
        Ok(())
    }
    fn append_tool_call_deltas(&mut self, value: &Value) -> Result<(), LlmError> {
        let Some(calls) = value.as_array() else {
            return Ok(());
        };
        for call in calls {
            let index = call["index"]
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| LlmError::Other("SSE tool_call 缺少 index".into()))?;
            while self.tool_calls.len() <= index {
                self.tool_calls.push(PartialToolCall::default());
            }
            let target = &mut self.tool_calls[index];
            if let Some(id) = call["id"].as_str() {
                target.id.push_str(id);
            }
            if let Some(name) = call["function"]["name"].as_str() {
                target.name.push_str(name);
            }
            if let Some(arguments) = call["function"]["arguments"].as_str() {
                target.arguments.push_str(arguments);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
