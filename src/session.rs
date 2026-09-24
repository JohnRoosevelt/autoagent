use crate::{
    agent::{Agent, AgentError, RetryPolicy, RunReport, StreamChatModel},
    llm::ToolCall,
    message::{Message, Role},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Duration};

/// Versioned, deterministic on-disk state for a resumable Agent session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Session {
    version: u8,
    messages: Vec<StoredMessage>,
    pending_inputs: Vec<String>,
    max_steps: usize,
    steps_taken: usize,
    retry_max_retries: usize,
    retry_initial_backoff_ms: u64,
    history_message_budget: usize,
    run_reports: Vec<RunReport>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct StoredMessage {
    role: Role,
    content: Option<String>,
    tool_calls: Vec<StoredToolCall>,
    tool_call_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct StoredToolCall {
    id: String,
    name: String,
    arguments: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("session I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("session JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported session version: {0}")]
    UnsupportedVersion(u8),
    #[error(transparent)]
    Agent(#[from] AgentError),
}

impl Session {
    pub fn capture<M: StreamChatModel>(agent: &Agent<M>) -> Self {
        let (
            messages,
            pending_inputs,
            max_steps,
            steps_taken,
            retry,
            history_message_budget,
            run_reports,
        ) = agent.session_data();
        Self {
            version: 1,
            messages: messages.into_iter().map(StoredMessage::from).collect(),
            pending_inputs,
            max_steps,
            steps_taken,
            retry_max_retries: retry.max_retries,
            retry_initial_backoff_ms: retry.initial_backoff.as_millis() as u64,
            history_message_budget,
            run_reports,
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SessionError> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, format!("{json}\n"))?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, SessionError> {
        let session: Self = serde_json::from_slice(&fs::read(path)?)?;
        session.validate()?;
        Ok(session)
    }

    pub fn restore<M: StreamChatModel>(&self, model: M) -> Result<Agent<M>, SessionError> {
        self.validate()?;
        Agent::from_session_data(
            model,
            self.messages.iter().cloned().map(Message::from).collect(),
            self.pending_inputs.clone(),
            self.max_steps,
            self.steps_taken,
            RetryPolicy::new(
                self.retry_max_retries,
                Duration::from_millis(self.retry_initial_backoff_ms),
            ),
            self.history_message_budget,
            self.run_reports.clone(),
        )
        .map_err(SessionError::from)
    }

    fn validate(&self) -> Result<(), SessionError> {
        if self.version != 1 {
            return Err(SessionError::UnsupportedVersion(self.version));
        }
        Ok(())
    }
}

impl From<Message> for StoredMessage {
    fn from(message: Message) -> Self {
        Self {
            role: message.role,
            content: message.content,
            tool_calls: message
                .tool_calls
                .into_iter()
                .map(|call| StoredToolCall {
                    id: call.id,
                    name: call.name,
                    arguments: call.arguments,
                })
                .collect(),
            tool_call_id: message.tool_call_id,
        }
    }
}

impl From<StoredMessage> for Message {
    fn from(message: StoredMessage) -> Self {
        Self {
            role: message.role,
            content: message.content,
            tool_calls: message
                .tool_calls
                .into_iter()
                .map(|call| ToolCall {
                    id: call.id,
                    name: call.name,
                    arguments: call.arguments,
                })
                .collect(),
            tool_call_id: message.tool_call_id,
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
