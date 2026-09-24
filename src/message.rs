use crate::llm::ToolCall;
use serde::Serialize;

/// OpenAI Chat Completions 协议支持的消息角色。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    /// 工具执行结果。本章仅保留消息形状，不产生此类消息。
    #[allow(dead_code)]
    Tool,
}

/// 一条可直接发送给 OpenAI-compatible 服务的对话消息。
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Message {
    pub role: Role,
    /// assistant 工具调用消息可以合法地使用 `null` content。
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// 仅 `role: tool` 需要；为第 06 章回填工具结果预留。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: Some(content.into()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }
    #[allow(dead_code)]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }

    pub fn assistant_with_tool_calls(content: Option<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content,
            tool_calls,
            tool_call_id: None,
        }
    }
}

/// 按发送顺序保存一段会话的消息历史。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Conversation {
    messages: Vec<Message>,
}

impl Conversation {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }
    pub fn add_system(&mut self, content: impl Into<String>) {
        self.push(Message::system(content));
    }
    pub fn add_user(&mut self, content: impl Into<String>) {
        self.push(Message::user(content));
    }
    #[allow(dead_code)]
    pub fn add_assistant(&mut self, content: impl Into<String>) {
        self.push(Message::assistant(content));
    }
    pub fn add_assistant_response(&mut self, content: String, tool_calls: Vec<ToolCall>) {
        self.push(Message::assistant_with_tool_calls(
            (!content.is_empty()).then_some(content),
            tool_calls,
        ));
    }
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod tests;
