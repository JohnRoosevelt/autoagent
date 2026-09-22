use serde::{Deserialize, Serialize};

/// OpenAI Chat Completions 协议支持的消息角色。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// 一条可直接发送给 OpenAI-compatible 服务的对话消息。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
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

    pub fn add_assistant(&mut self, content: impl Into<String>) {
        self.push(Message::assistant(content));
    }
}

#[cfg(test)]
#[path = "message_tests.rs"]
mod tests;
