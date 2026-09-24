use super::{
    Agent, AgentError, AgentEvent, AgentState, CancellationToken, RetryPolicy, StreamChatModel,
    TerminationReason,
};
use crate::{
    llm::{ChatResponse, FinishReason, LlmError, StreamEvent, ToolCall, ToolDefinition, Usage},
    message::{Conversation, Message, Role},
    tool::{GetWeatherTool, Tool, ToolError, ToolRegistry},
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    future::{Future, pending},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc;

#[derive(Clone)]
struct FakeModel {
    outcomes: Arc<Mutex<VecDeque<Outcome>>>,
    requests: Arc<Mutex<Vec<Vec<Message>>>>,
}

enum Outcome {
    Reply(ChatResponse),
    Error(LlmError),
    PartialThenError(String, LlmError),
    Pending,
}

impl FakeModel {
    fn new(outcomes: Vec<Outcome>) -> Self {
        Self {
            outcomes: Arc::new(Mutex::new(outcomes.into())),
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
        let outcome = self.outcomes.lock().unwrap().pop_front().unwrap();
        async move {
            match outcome {
                Outcome::Reply(response) => {
                    tx.send(StreamEvent::Start).await.unwrap();
                    if !response.content.is_empty() {
                        tx.send(StreamEvent::TextDelta(response.content.clone()))
                            .await
                            .unwrap();
                    }
                    tx.send(StreamEvent::Done(response.clone())).await.unwrap();
                    Ok(response)
                }
                Outcome::Error(error) => Err(error),
                Outcome::PartialThenError(delta, error) => {
                    tx.send(StreamEvent::Start).await.unwrap();
                    tx.send(StreamEvent::TextDelta(delta)).await.unwrap();
                    Err(error)
                }
                Outcome::Pending => {
                    tx.send(StreamEvent::Start).await.unwrap();
                    pending().await
                }
            }
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

fn zero_retry_policy(retries: usize) -> RetryPolicy {
    RetryPolicy::new(retries, Duration::ZERO)
}

#[test]
fn retry_policy_uses_deterministic_exponential_backoff() {
    let policy = RetryPolicy::new(3, Duration::from_millis(5));
    assert_eq!(policy.backoff_for_retry(1), Duration::from_millis(5));
    assert_eq!(policy.backoff_for_retry(2), Duration::from_millis(10));
    assert_eq!(policy.backoff_for_retry(3), Duration::from_millis(20));
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
        FakeModel::new(vec![Outcome::Reply(text("一")), Outcome::Reply(text("二"))]),
        Conversation::new(),
        3,
    )
    .unwrap();
    agent.enqueue_user("第一问");
    agent.enqueue_user("第二问");
    let (tx, mut rx) = mpsc::channel(64);
    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.termination, TerminationReason::NoPendingInput);
    assert_eq!(report.steps_taken, 2);
    assert_eq!(report.model_attempts, 2);
    assert_eq!(report.retries, 0);
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
    assert_eq!(rx.recv().await, Some(AgentEvent::Started));
    assert_eq!(
        rx.recv().await,
        Some(AgentEvent::Input {
            content: "第一问".into()
        })
    );
    assert_eq!(rx.recv().await, Some(AgentEvent::Attempt { attempt: 1 }));
    assert_eq!(rx.recv().await, Some(AgentEvent::Model(StreamEvent::Start)));
    assert_eq!(
        rx.recv().await,
        Some(AgentEvent::Model(StreamEvent::TextDelta("一".into())))
    );
}

#[tokio::test]
async fn retries_a_retryable_error_then_records_only_the_successful_response() {
    let model = FakeModel::new(vec![
        Outcome::PartialThenError("不完整".into(), LlmError::Network("reset".into())),
        Outcome::Reply(text("完整回答")),
    ]);
    let mut agent = Agent::new(model.clone(), Conversation::new(), 2)
        .unwrap()
        .with_retry_policy(zero_retry_policy(1));
    agent.enqueue_user("问题");
    let (tx, mut rx) = mpsc::channel(64);

    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.steps_taken, 1);
    assert_eq!(report.model_attempts, 2);
    assert_eq!(report.retries, 1);
    assert_eq!(model.requests().len(), 2);
    assert_eq!(
        agent.conversation().messages(),
        [Message::user("问题"), Message::assistant("完整回答")]
    );
    let events: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(events.contains(&AgentEvent::Model(StreamEvent::TextDelta("不完整".into()))));
    assert!(events.contains(&AgentEvent::Retry {
        attempt: 2,
        delay: Duration::ZERO
    }));
}

#[tokio::test]
async fn does_not_retry_non_retryable_errors() {
    let model = FakeModel::new(vec![Outcome::Error(LlmError::from_http(
        401,
        "bad key".into(),
    ))]);
    let mut agent = Agent::new(model.clone(), Conversation::new(), 2)
        .unwrap()
        .with_retry_policy(zero_retry_policy(3));
    agent.enqueue_user("问题");
    let (tx, mut rx) = mpsc::channel(64);

    let error = agent.run(tx).await.unwrap_err();
    assert!(matches!(
        error,
        AgentError::Model(LlmError::InvalidRequest { status: 401, .. })
    ));
    let events: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(events.iter().any(|event| matches!(
        event,
        AgentEvent::Failed { error } if error.contains("请求无效")
    )));
    assert_eq!(model.requests().len(), 1);
    assert_eq!(agent.state(), AgentState::Failed);
    assert_eq!(agent.conversation().messages(), [Message::user("问题")]);
}

#[tokio::test]
async fn retry_exhaustion_preserves_the_source_and_does_not_fabricate_messages() {
    let model = FakeModel::new(vec![
        Outcome::Error(LlmError::from_http(429, "slow down".into())),
        Outcome::Error(LlmError::Network("reset".into())),
    ]);
    let mut agent = Agent::new(model.clone(), Conversation::new(), 2)
        .unwrap()
        .with_retry_policy(zero_retry_policy(1));
    agent.enqueue_user("问题");
    let (tx, _rx) = mpsc::channel(64);

    let error = agent.run(tx).await.unwrap_err();
    assert!(matches!(
        error,
        AgentError::RetriesExhausted {
            attempts: 2,
            source: LlmError::Network(_)
        }
    ));
    assert_eq!(model.requests().len(), 2);
    assert_eq!(agent.conversation().messages(), [Message::user("问题")]);
}

#[tokio::test]
async fn cancellation_during_retry_backoff_stops_before_the_next_attempt() {
    let model = FakeModel::new(vec![Outcome::Error(LlmError::Network("reset".into()))]);
    let token = CancellationToken::new();
    let mut agent = Agent::new(model.clone(), Conversation::new(), 2)
        .unwrap()
        .with_retry_policy(RetryPolicy::new(2, Duration::from_secs(60)));
    agent.enqueue_user("问题");
    let (tx, mut rx) = mpsc::channel(64);
    let report = {
        let run = agent.run_with_cancellation(tx, &token);
        tokio::pin!(run);
        loop {
            let event = tokio::select! {
                event = rx.recv() => event,
                result = &mut run => panic!("Agent unexpectedly completed: {result:?}"),
            };
            if event
                == Some(AgentEvent::Retry {
                    attempt: 2,
                    delay: Duration::from_secs(60),
                })
            {
                break;
            }
        }
        token.cancel();
        run.as_mut().await.unwrap()
    };
    assert_eq!(report.termination, TerminationReason::Cancelled);
    assert_eq!(report.model_attempts, 1);
    assert_eq!(report.retries, 1);
    assert_eq!(model.requests().len(), 1);
    assert_eq!(
        agent.state(),
        AgentState::Stopped(TerminationReason::Cancelled)
    );
    assert_eq!(rx.recv().await, Some(AgentEvent::Cancelled));
}

#[tokio::test]
async fn cancellation_during_a_model_stream_aborts_it_without_an_assistant_message() {
    let model = FakeModel::new(vec![Outcome::Pending]);
    let token = CancellationToken::new();
    let mut agent = Agent::new(model.clone(), Conversation::new(), 2).unwrap();
    agent.enqueue_user("问题");
    let (tx, mut rx) = mpsc::channel(64);
    let report = {
        let run = agent.run_with_cancellation(tx, &token);
        tokio::pin!(run);
        loop {
            let event = tokio::select! {
                event = rx.recv() => event,
                result = &mut run => panic!("Agent unexpectedly completed: {result:?}"),
            };
            if event == Some(AgentEvent::Model(StreamEvent::Start)) {
                break;
            }
        }
        token.cancel();
        run.as_mut().await.unwrap()
    };
    assert_eq!(report.termination, TerminationReason::Cancelled);
    assert_eq!(report.steps_taken, 0);
    assert_eq!(report.model_attempts, 1);
    assert_eq!(agent.conversation().messages(), [Message::user("问题")]);
    assert_eq!(rx.recv().await, Some(AgentEvent::Cancelled));
}

struct CancellingTool {
    token: CancellationToken,
    calls: Arc<Mutex<usize>>,
}

impl Tool for CancellingTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new("cancel_after_first", "测试取消", json!({"type":"object"}))
    }

    fn execute(&self, _arguments: Value) -> Result<Value, ToolError> {
        *self.calls.lock().unwrap() += 1;
        self.token.cancel();
        Ok(json!({"completed": true}))
    }
}

#[tokio::test]
async fn cancellation_while_forwarding_to_a_backpressured_event_channel_aborts_the_model() {
    let token = CancellationToken::new();
    let mut agent = Agent::new(
        FakeModel::new(vec![Outcome::Reply(text("完整但不应入账"))]),
        Conversation::new(),
        2,
    )
    .unwrap();
    agent.enqueue_user("问题");
    let (tx, mut rx) = mpsc::channel(1);
    let report = {
        let run = agent.run_with_cancellation(tx, &token);
        tokio::pin!(run);
        let event = tokio::select! {
            event = rx.recv() => event,
            result = &mut run => panic!("Agent unexpectedly completed: {result:?}"),
        };
        assert_eq!(event, Some(AgentEvent::Started));
        tokio::task::yield_now().await;
        token.cancel();
        run.as_mut().await.unwrap()
    };
    assert_eq!(report.termination, TerminationReason::Cancelled);
    assert_eq!(report.steps_taken, 0);
    assert_eq!(agent.conversation().messages(), [Message::user("问题")]);
}

#[tokio::test]
async fn closed_event_receiver_fails_the_agent_and_aborts_the_model_task() {
    let mut agent = Agent::new(
        FakeModel::new(vec![Outcome::Pending]),
        Conversation::new(),
        2,
    )
    .unwrap();
    agent.enqueue_user("问题");
    let (tx, rx) = mpsc::channel(1);
    drop(rx);

    assert!(matches!(
        agent.run(tx).await,
        Err(AgentError::EventReceiverClosed)
    ));
    assert_eq!(agent.state(), AgentState::Failed);
}

#[tokio::test]
async fn cancellation_after_one_tool_prevents_later_tools_and_the_next_model_call() {
    let token = CancellationToken::new();
    let executions = Arc::new(Mutex::new(0));
    let mut registry = ToolRegistry::new();
    registry
        .register(CancellingTool {
            token: token.clone(),
            calls: executions.clone(),
        })
        .unwrap();
    let model = FakeModel::new(vec![Outcome::Reply(calls(vec![
        tool_call("call_1", "cancel_after_first", "{}"),
        tool_call("call_2", "cancel_after_first", "{}"),
    ]))]);
    let mut agent = Agent::new(model.clone(), Conversation::new(), 3)
        .unwrap()
        .with_tool_registry(registry);
    agent.enqueue_user("执行");
    let (tx, mut rx) = mpsc::channel(64);

    let report = agent.run_with_cancellation(tx, &token).await.unwrap();
    assert_eq!(report.termination, TerminationReason::Cancelled);
    assert_eq!(report.steps_taken, 1);
    assert_eq!(report.tool_calls_processed, 1);
    assert_eq!(*executions.lock().unwrap(), 1);
    assert_eq!(model.requests().len(), 1);
    assert_eq!(agent.conversation().messages().len(), 3);
    assert_eq!(
        agent.conversation().messages()[2].tool_call_id.as_deref(),
        Some("call_1")
    );
    while rx.recv().await != Some(AgentEvent::Cancelled) {}
}

#[tokio::test]
async fn completes_a_tool_round_and_associates_the_result_with_its_call() {
    let call = tool_call("call_1", "get_weather", r#"{"city":"北京"}"#);
    let model = FakeModel::new(vec![
        Outcome::Reply(calls(vec![call.clone()])),
        Outcome::Reply(text("北京天气是晴，20°C。")),
    ]);
    let mut agent = Agent::new(model.clone(), Conversation::new(), 3)
        .unwrap()
        .with_tool_registry(weather_registry());
    agent.enqueue_user("北京天气？");
    let (tx, mut rx) = mpsc::channel(64);
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
        agent.conversation().messages()[3],
        Message::assistant("北京天气是晴，20°C。")
    );
    assert_eq!(model.requests().len(), 2);
    let events: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(events.contains(&AgentEvent::ToolStarted {
        call_id: "call_1".into(),
        name: "get_weather".into(),
    }));
    assert!(events.contains(&AgentEvent::ToolFinished {
        call_id: "call_1".into(),
        name: "get_weather".into(),
    }));
    assert!(events.contains(&AgentEvent::Finished(report)));
}

#[tokio::test]
async fn executes_multiple_tool_calls_serially_in_model_order() {
    let first = tool_call("call_1", "get_weather", r#"{"city":"北京"}"#);
    let second = tool_call("call_2", "get_weather", r#"{"city":"上海"}"#);
    let mut agent = Agent::new(
        FakeModel::new(vec![
            Outcome::Reply(calls(vec![first.clone(), second.clone()])),
            Outcome::Reply(text("已比较天气。")),
        ]),
        Conversation::new(),
        3,
    )
    .unwrap()
    .with_tool_registry(weather_registry());
    agent.enqueue_user("比较北京和上海天气");
    let (tx, _rx) = mpsc::channel(64);
    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.tool_calls_processed, 2);
    let messages = agent.conversation().messages();
    assert_eq!(
        messages[1],
        Message::assistant_with_tool_calls(None, vec![first, second])
    );
    assert_eq!(messages[2].tool_call_id.as_deref(), Some("call_1"));
    assert_eq!(messages[3].tool_call_id.as_deref(), Some("call_2"));
}

#[tokio::test]
async fn returns_tool_failures_to_the_model_and_continues() {
    let invalid_json = tool_call("bad_json", "get_weather", "{");
    let missing_city = tool_call("missing_city", "get_weather", "{}");
    let unknown = tool_call("unknown", "not_registered", "{}");
    let mut agent = Agent::new(
        FakeModel::new(vec![
            Outcome::Reply(calls(vec![invalid_json, missing_city, unknown])),
            Outcome::Reply(text("请改正工具参数后重试。")),
        ]),
        Conversation::new(),
        3,
    )
    .unwrap()
    .with_tool_registry(weather_registry());
    agent.enqueue_user("查询天气");
    let (tx, _rx) = mpsc::channel(64);
    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.tool_calls_processed, 3);
    let messages = agent.conversation().messages();
    for (message, code) in
        messages[2..5]
            .iter()
            .zip(["invalid_json", "invalid_arguments", "unknown_tool"])
    {
        let content: Value = serde_json::from_str(message.content.as_deref().unwrap()).unwrap();
        assert_eq!(content["ok"], false);
        assert_eq!(content["error"]["code"], code);
    }
}

#[tokio::test]
async fn max_steps_counts_completed_model_turns_but_still_records_tool_results() {
    let mut agent = Agent::new(
        FakeModel::new(vec![Outcome::Reply(calls(vec![tool_call(
            "call_1",
            "get_weather",
            r#"{"city":"北京"}"#,
        )]))]),
        Conversation::new(),
        1,
    )
    .unwrap()
    .with_tool_registry(weather_registry());
    agent.enqueue_user("北京天气？");
    let (tx, _rx) = mpsc::channel(64);
    let report = agent.run(tx).await.unwrap();
    assert_eq!(report.termination, TerminationReason::MaxStepsReached);
    assert_eq!(report.steps_taken, 1);
    assert_eq!(report.tool_calls_processed, 1);
    assert_eq!(agent.conversation().messages().len(), 3);
}
