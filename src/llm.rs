pub mod client;

use serde::{Serialize, Serializer, ser::SerializeStruct};
use serde_json::Value;
use std::time::Duration;

/// 宿主程序可提供给 OpenAI-compatible 模型的函数定义。
#[derive(Clone, Debug, PartialEq)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    /// OpenAI `function.parameters` 使用的 JSON Schema；本章不验证它。
    pub parameters: Value,
}

#[allow(dead_code)]
impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

/// 模型请求的一次函数调用。`arguments` 保留 provider 返回的原始 JSON 字符串。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl Serialize for ToolCall {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ToolCall", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("type", "function")?;
        state.serialize_field(
            "function",
            &serde_json::json!({ "name": self.name, "arguments": self.arguments }),
        )?;
        state.end()
    }
}

/// 模型结束当前回复的原因。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Other(String),
}

impl Default for FinishReason {
    fn default() -> Self {
        Self::Other("missing".into())
    }
}

/// 一次模型调用的 token 计量。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

/// 当前这轮模型调用的完整结果，包括文本、工具意图与服务端计量。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    pub usage: Usage,
}

/// 由 LLM 流式协议转换而来的领域事件。
///
/// 调用方不必了解 provider 的 SSE 格式。工具调用在 delta 间通常不完整，故只在
/// `Done` 的完整 `ChatResponse` 中暴露，避免显示层解析或拼接原始 JSON。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamEvent {
    Start,
    TextDelta(String),
    Done(ChatResponse),
    /// Agent 将再次发起模型请求；`attempt` 为即将开始的第几次尝试。
    Retrying {
        attempt: usize,
        delay: Duration,
    },
    /// Agent 收到应用内协作式取消请求后停止运行。
    Cancelled,
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("配置错误: {0}")]
    Config(String),
    #[error("网络错误: {0}")]
    Network(String),
    #[error("请求无效（4xx，重试没有意义）: {body}")]
    InvalidRequest { status: u16, body: String },
    #[error("可重试的 HTTP 错误: {status} {body}")]
    Server { status: u16, body: String },
    #[error("其他错误: {0}")]
    Other(String),
}

impl LlmError {
    pub fn from_http(status: u16, body: String) -> Self {
        match status {
            408 | 429 => LlmError::Server { status, body },
            400..=499 => LlmError::InvalidRequest { status, body },
            500..=599 => LlmError::Server { status, body },
            _ => LlmError::Other(format!("HTTP {status}: {body}")),
        }
    }

    #[allow(dead_code)]
    pub fn is_retryable(&self) -> bool {
        matches!(self, LlmError::Network(_) | LlmError::Server { .. })
    }
}

#[cfg(test)]
mod tests;
