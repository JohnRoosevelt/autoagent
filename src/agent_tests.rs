use super::{Agent, AgentError, AgentState, StreamChatModel, TerminationReason};
use crate::{
    llm::{ChatResponse, LlmError, StreamEvent, Usage},
    message::{Conversation, Message},
};
use std::{future::Future, sync::Arc};
use tokio::sync::mpsc;

#[derive(Clone)]
struct FakeModel {
    replies: Arc<Vec<String>>,
}

impl FakeModel {
    fn with_replies(replies: &[&str]) -> Self {
        Self {
            replies: Arc::new(replies.iter().map(|reply| (*reply).to_owned()).collect()),
        }
    }
}

impl StreamChatModel for FakeModel {
    fn chat_stream(
        &self,
        messages: &[Message],
        tx: mpsc::Sender<StreamEvent>,
    ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send {
        let reply = self.replies[messages
            .iter()
            .filter(|message| message.role == crate::message::Role::User)
            .count()
            - 1]
        .clone();
        async move {
            tx.send(StreamEvent::Start).await.unwrap();
            tx.send(StreamEvent::TextDelta(reply.clone()))
                .await
                .unwrap();
            let response = ChatResponse {
                content: reply,
                usage: Usage::default(),
            };
            tx.send(StreamEvent::Done(response.clone())).await.unwrap();
            Ok(response)
        }
    }
}

#[test]
fn rejects_zero_max_steps() {
    let result = Agent::new(FakeModel::with_replies(&[]), Conversation::new(), 0);

    assert!(matches!(result, Err(AgentError::InvalidMaxSteps)));
}

#[tokio::test]
async fn updates_history_and_forwards_stream_events_for_each_step() {
    let mut conversation = Conversation::new();
    conversation.add_system("你是一个助手。");
    let mut agent = Agent::new(
        FakeModel::with_replies(&["你好，小赤。", "你的名字是小赤。"]),
        conversation,
        3,
    )
    .unwrap();
    agent.enqueue_user("我叫小赤。");
    agent.enqueue_user("我叫什么？");
    assert_eq!(agent.state(), AgentState::Ready);

    let (tx, mut rx) = mpsc::channel(8);
    let report = agent.run(tx).await.unwrap();

    assert_eq!(report.steps_taken, 2);
    assert_eq!(report.termination, TerminationReason::NoPendingInput);
    assert_eq!(
        agent.state(),
        AgentState::Stopped(TerminationReason::NoPendingInput)
    );
    assert_eq!(agent.pending_input_count(), 0);
    assert_eq!(
        agent.conversation().messages(),
        [
            Message::system("你是一个助手。"),
            Message::user("我叫小赤。"),
            Message::assistant("你好，小赤。"),
            Message::user("我叫什么？"),
            Message::assistant("你的名字是小赤。"),
        ]
    );

    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    assert_eq!(
        events,
        [
            StreamEvent::Start,
            StreamEvent::TextDelta("你好，小赤。".into()),
            StreamEvent::Done(ChatResponse {
                content: "你好，小赤。".into(),
                usage: Usage::default()
            }),
            StreamEvent::Start,
            StreamEvent::TextDelta("你的名字是小赤。".into()),
            StreamEvent::Done(ChatResponse {
                content: "你的名字是小赤。".into(),
                usage: Usage::default()
            }),
        ]
    );
}

#[tokio::test]
async fn stops_at_max_steps_without_consuming_later_input() {
    let mut agent = Agent::new(
        FakeModel::with_replies(&["一", "二", "三"]),
        Conversation::new(),
        2,
    )
    .unwrap();
    agent.enqueue_user("第一问");
    agent.enqueue_user("第二问");
    agent.enqueue_user("第三问");

    let (tx, _rx) = mpsc::channel(16);
    let report = agent.run(tx).await.unwrap();

    assert_eq!(report.steps_taken, 2);
    assert_eq!(report.termination, TerminationReason::MaxStepsReached);
    assert_eq!(
        agent.state(),
        AgentState::Stopped(TerminationReason::MaxStepsReached)
    );
    assert_eq!(agent.pending_input_count(), 1);
    assert_eq!(
        agent.conversation().messages(),
        [
            Message::user("第一问"),
            Message::assistant("一"),
            Message::user("第二问"),
            Message::assistant("二"),
        ]
    );
}
