use super::{Agent, AgentError, AgentState, StreamChatModel, TerminationReason};
use crate::{
    llm::{ChatResponse, FinishReason, LlmError, StreamEvent, ToolCall, ToolDefinition, Usage},
    message::{Conversation, Message},
};
use std::{future::Future, sync::Arc};
use tokio::sync::mpsc;

#[derive(Clone)]
struct FakeModel {
    replies: Arc<Vec<ChatResponse>>,
}
impl FakeModel {
    fn new(replies: Vec<ChatResponse>) -> Self {
        Self {
            replies: Arc::new(replies),
        }
    }
}
impl StreamChatModel for FakeModel {
    fn chat_stream(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
        tx: mpsc::Sender<StreamEvent>,
    ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send {
        let response = self.replies[messages
            .iter()
            .filter(|message| message.role == crate::message::Role::User)
            .count()
            - 1]
        .clone();
        async move {
            tx.send(StreamEvent::Start).await.unwrap();
            if !response.content.is_empty() {
                tx.send(StreamEvent::TextDelta(response.content.clone()))
                    .await
                    .unwrap();
            }
            tx.send(StreamEvent::Done(response.clone())).await.unwrap();
            Ok(response)
        }
    }
}
fn text(content: &str) -> ChatResponse {
    ChatResponse {
        content: content.into(),
        tool_calls: vec![],
        finish_reason: FinishReason::Stop,
        usage: Usage::default(),
    }
}

#[test]
fn rejects_zero_max_steps() {
    assert!(matches!(
        Agent::new(FakeModel::new(vec![]), Conversation::new(), 0),
        Err(AgentError::InvalidMaxSteps)
    ));
}

#[tokio::test]
async fn updates_history_and_forwards_stream_events_for_each_step() {
    let mut agent = Agent::new(
        FakeModel::new(vec![text("一"), text("二")]),
        Conversation::new(),
        3,
    )
    .unwrap();
    agent.enqueue_user("第一问");
    agent.enqueue_user("第二问");
    let (tx, mut rx) = mpsc::channel(8);
    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.termination, TerminationReason::NoPendingInput);
    assert_eq!(report.steps_taken, 2);
    assert_eq!(
        agent.conversation().messages(),
        [
            Message::user("第一问"),
            Message::assistant("一"),
            Message::user("第二问"),
            Message::assistant("二")
        ]
    );
    assert_eq!(rx.recv().await, Some(StreamEvent::Start));
    assert_eq!(rx.recv().await, Some(StreamEvent::TextDelta("一".into())));
}

#[tokio::test]
async fn stops_and_preserves_assistant_context_when_tools_are_requested() {
    let call = ToolCall {
        id: "call_1".into(),
        name: "weather".into(),
        arguments: "{\"city\":\"北京\"}".into(),
    };
    let response = ChatResponse {
        content: String::new(),
        tool_calls: vec![call.clone()],
        finish_reason: FinishReason::ToolCalls,
        usage: Usage::default(),
    };
    let mut agent = Agent::new(FakeModel::new(vec![response]), Conversation::new(), 3)
        .unwrap()
        .with_tools(vec![ToolDefinition::new(
            "weather",
            "查询天气",
            serde_json::json!({}),
        )]);
    agent.enqueue_user("北京天气？");
    agent.enqueue_user("不会执行");
    let (tx, _rx) = mpsc::channel(8);
    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.termination, TerminationReason::ToolCallsRequested);
    assert_eq!(report.requested_tool_calls, vec![call.clone()]);
    assert_eq!(
        agent.state(),
        AgentState::Stopped(TerminationReason::ToolCallsRequested)
    );
    assert_eq!(agent.pending_input_count(), 1);
    assert_eq!(
        agent.conversation().messages(),
        [
            Message::user("北京天气？"),
            Message::assistant_with_tool_calls(None, vec![call])
        ]
    );
}
