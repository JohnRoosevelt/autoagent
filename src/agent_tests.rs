use super::{Agent, AgentError, AgentState, StreamChatModel, TerminationReason};
use crate::{
    llm::{ChatResponse, FinishReason, LlmError, StreamEvent, ToolCall, ToolDefinition, Usage},
    message::{Conversation, Message, Role},
    tool::{GetWeatherTool, ToolRegistry},
};
use std::{
    collections::VecDeque,
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;

#[derive(Clone)]
struct FakeModel {
    replies: Arc<Mutex<VecDeque<ChatResponse>>>,
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
}
impl FakeModel {
    fn new(replies: Vec<ChatResponse>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into())),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }
    fn requests(&self) -> Vec<Vec<Message>> {
        self.requests.lock().unwrap().clone()
    }
}
impl StreamChatModel for FakeModel {
    fn chat_stream(
        &self,
        messages: &[Message],
        _tools: &[ToolDefinition],
        tx: mpsc::Sender<StreamEvent>,
    ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send {
        self.requests.lock().unwrap().push(messages.to_vec());
        let response = self.replies.lock().unwrap().pop_front().unwrap();
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
fn tool_call(id: &str, name: &str, arguments: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: name.into(),
        arguments: arguments.into(),
    }
}
fn calls(calls: Vec<ToolCall>) -> ChatResponse {
    ChatResponse {
        content: String::new(),
        tool_calls: calls,
        finish_reason: FinishReason::ToolCalls,
        usage: Usage::default(),
    }
}
fn weather_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(GetWeatherTool).unwrap();
    registry
}

#[test]
fn rejects_zero_max_steps() {
    assert!(matches!(
        Agent::new(FakeModel::new(vec![]), Conversation::new(), 0),
        Err(AgentError::InvalidMaxSteps)
    ));
}

#[tokio::test]
async fn updates_history_and_forwards_stream_events_for_each_input() {
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
    assert_eq!(report.tool_calls_processed, 0);
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
async fn completes_a_tool_round_and_associates_the_result_with_its_call() {
    let call = tool_call("call_1", "get_weather", r#"{"city":"北京"}"#);
    let model = FakeModel::new(vec![
        calls(vec![call.clone()]),
        text("北京天气是晴，20°C。"),
    ]);
    let mut agent = Agent::new(model.clone(), Conversation::new(), 3)
        .unwrap()
        .with_tool_registry(weather_registry());
    agent.enqueue_user("北京天气？");

    let (tx, _rx) = mpsc::channel(8);
    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.termination, TerminationReason::NoPendingInput);
    assert_eq!(report.steps_taken, 2);
    assert_eq!(report.tool_calls_processed, 1);
    assert_eq!(
        agent.state(),
        AgentState::Stopped(TerminationReason::NoPendingInput)
    );
    assert_eq!(
        agent.conversation().messages()[1],
        Message::assistant_with_tool_calls(None, vec![call])
    );
    assert_eq!(agent.conversation().messages()[2].role, Role::Tool);
    assert_eq!(
        agent.conversation().messages()[2].tool_call_id.as_deref(),
        Some("call_1")
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(
            agent.conversation().messages()[2]
                .content
                .as_deref()
                .unwrap()
        )
        .unwrap()["result"]["city"],
        "北京"
    );
    assert_eq!(
        agent.conversation().messages()[3],
        Message::assistant("北京天气是晴，20°C。")
    );
    assert_eq!(model.requests().len(), 2);
    assert_eq!(model.requests()[1][1].role, Role::Assistant);
    assert_eq!(model.requests()[1][2].role, Role::Tool);
}

#[tokio::test]
async fn executes_multiple_tool_calls_serially_in_model_order() {
    let first = tool_call("call_1", "get_weather", r#"{"city":"北京"}"#);
    let second = tool_call("call_2", "get_weather", r#"{"city":"上海"}"#);
    let mut agent = Agent::new(
        FakeModel::new(vec![
            calls(vec![first.clone(), second.clone()]),
            text("已比较天气。"),
        ]),
        Conversation::new(),
        3,
    )
    .unwrap()
    .with_tool_registry(weather_registry());
    agent.enqueue_user("比较北京和上海天气");
    let (tx, _rx) = mpsc::channel(8);

    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.tool_calls_processed, 2);
    let messages = agent.conversation().messages();
    assert_eq!(
        messages[1],
        Message::assistant_with_tool_calls(None, vec![first, second])
    );
    assert_eq!(messages[2].tool_call_id.as_deref(), Some("call_1"));
    assert_eq!(messages[3].tool_call_id.as_deref(), Some("call_2"));
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(messages[2].content.as_deref().unwrap()).unwrap()
            ["result"]["city"],
        "北京"
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(messages[3].content.as_deref().unwrap()).unwrap()
            ["result"]["city"],
        "上海"
    );
}

#[tokio::test]
async fn returns_tool_failures_to_the_model_and_continues() {
    let invalid_json = tool_call("bad_json", "get_weather", "{");
    let missing_city = tool_call("missing_city", "get_weather", "{}");
    let unknown = tool_call("unknown", "not_registered", "{}");
    let mut agent = Agent::new(
        FakeModel::new(vec![
            calls(vec![invalid_json, missing_city, unknown]),
            text("请改正工具参数后重试。"),
        ]),
        Conversation::new(),
        3,
    )
    .unwrap()
    .with_tool_registry(weather_registry());
    agent.enqueue_user("查询天气");
    let (tx, _rx) = mpsc::channel(8);

    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.tool_calls_processed, 3);
    let messages = agent.conversation().messages();
    for (message, code) in
        messages[2..5]
            .iter()
            .zip(["invalid_json", "invalid_arguments", "unknown_tool"])
    {
        let content: serde_json::Value =
            serde_json::from_str(message.content.as_deref().unwrap()).unwrap();
        assert_eq!(content["ok"], false);
        assert_eq!(content["error"]["code"], code);
    }
    assert_eq!(messages[5], Message::assistant("请改正工具参数后重试。"));
}

#[tokio::test]
async fn max_steps_counts_model_calls_but_still_records_tool_results() {
    let mut agent = Agent::new(
        FakeModel::new(vec![calls(vec![tool_call(
            "call_1",
            "get_weather",
            r#"{"city":"北京"}"#,
        )])]),
        Conversation::new(),
        1,
    )
    .unwrap()
    .with_tool_registry(weather_registry());
    agent.enqueue_user("北京天气？");
    let (tx, _rx) = mpsc::channel(8);

    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.termination, TerminationReason::MaxStepsReached);
    assert_eq!(report.steps_taken, 1);
    assert_eq!(report.tool_calls_processed, 1);
    assert_eq!(agent.conversation().messages().len(), 3);
}
