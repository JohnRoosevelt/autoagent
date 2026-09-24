use super::Session;
use crate::{
    agent::{Agent, StreamChatModel},
    llm::{ChatResponse, LlmError, StreamEvent, ToolDefinition, Usage},
    message::Conversation,
};
use std::{future::Future, path::PathBuf};
use tokio::sync::mpsc;

#[derive(Clone)]
struct NoopModel;
impl StreamChatModel for NoopModel {
    fn chat_stream(
        &self,
        _: &[crate::message::Message],
        _: &[ToolDefinition],
        _: mpsc::Sender<StreamEvent>,
    ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send {
        async {
            Ok(ChatResponse {
                content: String::new(),
                tool_calls: vec![],
                finish_reason: Default::default(),
                usage: Usage::default(),
            })
        }
    }
}

fn session_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "autoagent-session-{name}-{}.json",
        std::process::id()
    ))
}

#[test]
fn saves_deterministic_json_and_restores_pending_state() {
    let mut conversation = Conversation::new();
    conversation.add_system("system");
    conversation.add_user("completed input");
    let mut agent = Agent::new(NoopModel, conversation, 4)
        .unwrap()
        .with_history_message_budget(7);
    agent.enqueue_user("queued input");
    let session = Session::capture(&agent);
    let path = session_path("round-trip");
    session.save(&path).unwrap();
    let first = std::fs::read_to_string(&path).unwrap();
    session.save(&path).unwrap();
    assert_eq!(first, std::fs::read_to_string(&path).unwrap());

    let restored = Session::load(&path).unwrap().restore(NoopModel).unwrap();
    assert_eq!(
        restored.conversation().messages(),
        agent.conversation().messages()
    );
    assert_eq!(restored.pending_input_count(), 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn rejects_unknown_session_version() {
    let path = session_path("version");
    let session = Session::capture(&Agent::new(NoopModel, Conversation::new(), 1).unwrap());
    let invalid = serde_json::to_string(&session)
        .unwrap()
        .replace("\"version\":1", "\"version\":2");
    std::fs::write(&path, invalid).unwrap();
    assert!(matches!(
        Session::load(&path),
        Err(super::SessionError::UnsupportedVersion(2))
    ));
    let _ = std::fs::remove_file(path);
}
