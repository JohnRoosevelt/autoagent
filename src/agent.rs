use crate::{
    context::ContextManager,
    llm::{ChatResponse, LlmError, StreamEvent, ToolDefinition},
    message::{Conversation, Message},
    tool::{ToolError, ToolRegistry},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::VecDeque,
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::{Notify, mpsc};

/// Agent 依赖的最小模型能力；实现不需要暴露 SSE 协议细节。
pub trait StreamChatModel: Clone + Send + Sync + 'static {
    fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        tx: mpsc::Sender<StreamEvent>,
    ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send;
}

/// 应用内协作式取消令牌。取消不会抢占已开始的同步本地工具。
#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    inner: Arc<CancellationInner>,
}

#[derive(Debug, Default)]
struct CancellationInner {
    cancelled: AtomicBool,
    notify: Notify,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(dead_code)]
    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::Release);
        self.inner.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }

    async fn cancelled(&self) {
        loop {
            let notified = self.inner.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

/// 有限、确定性的模型重试策略。第 07 章不使用 jitter 或全局限流。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetryPolicy {
    /// 首次失败后最多再发起的模型尝试数。
    pub max_retries: usize,
    pub initial_backoff: Duration,
}

impl RetryPolicy {
    pub fn new(max_retries: usize, initial_backoff: Duration) -> Self {
        Self {
            max_retries,
            initial_backoff,
        }
    }

    /// `retry_index` 从 1 开始；退避为 initial, initial * 2, initial * 4 ...。
    pub fn backoff_for_retry(&self, retry_index: usize) -> Duration {
        let multiplier = 1u32
            .checked_shl((retry_index.saturating_sub(1)) as u32)
            .unwrap_or(u32::MAX);
        self.initial_backoff.saturating_mul(multiplier)
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(2, Duration::from_millis(250))
    }
}

/// 一次 Agent 运行结束的明确原因。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TerminationReason {
    NoPendingInput,
    MaxStepsReached,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentState {
    Ready,
    Running,
    Stopped(TerminationReason),
    Failed,
}

/// Agent 对宿主程序发出的最小运行生命周期事件。
///
/// `Model` 保留 LLM 层的流式边界；其余变体描述 Agent 对输入、重试、账本和工具的编排。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentEvent {
    Started,
    Input { content: String },
    Attempt { attempt: usize },
    Retry { attempt: usize, delay: Duration },
    Model(StreamEvent),
    AssistantRecorded { response: ChatResponse },
    ToolStarted { call_id: String, name: String },
    ToolFinished { call_id: String, name: String },
    ContextTrimmed { removed_messages: usize },
    Cancelled,
    Finished(RunReport),
    Failed { error: String },
}

/// 一次运行的摘要，供调用方记录和显示。
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunReport {
    /// 完整成功并已处理响应的模型回合数；失败尝试与工具执行不单独计 step。
    pub steps_taken: usize,
    pub termination: TerminationReason,
    /// 本次已被 Registry 串行处理（成功或失败）的 tool call 数量。
    pub tool_calls_processed: usize,
    /// 实际发起的模型请求总数，包含失败后重试。
    pub model_attempts: usize,
    /// 已执行的退避重试次数。
    pub retries: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent 配置错误: max_steps 必须大于 0")]
    InvalidMaxSteps,
    #[error("模型重试耗尽（已尝试 {attempts} 次）: {source}")]
    RetriesExhausted {
        attempts: usize,
        #[source]
        source: LlmError,
    },
    #[error(transparent)]
    Model(#[from] LlmError),
    #[error("Agent 事件接收方已关闭")]
    EventReceiverClosed,
    #[error("Agent 模型任务失败: {0}")]
    ModelTask(String),
}

/// 管理一次运行的会话、运行时本地工具、待处理输入和循环状态。
pub struct Agent<M> {
    model: M,
    conversation: Conversation,
    tools: ToolRegistry,
    pending_inputs: VecDeque<String>,
    max_steps: usize,
    steps_taken: usize,
    retry_policy: RetryPolicy,
    context_manager: ContextManager,
    state: AgentState,
    run_reports: Vec<RunReport>,
}

impl<M: StreamChatModel> Agent<M> {
    pub fn new(model: M, conversation: Conversation, max_steps: usize) -> Result<Self, AgentError> {
        if max_steps == 0 {
            return Err(AgentError::InvalidMaxSteps);
        }
        Ok(Self {
            model,
            conversation,
            tools: ToolRegistry::new(),
            pending_inputs: VecDeque::new(),
            max_steps,
            steps_taken: 0,
            retry_policy: RetryPolicy::default(),
            context_manager: ContextManager::new(usize::MAX),
            state: AgentState::Ready,
            run_reports: Vec::new(),
        })
    }

    /// 注入启动时组装的本地 Registry；该 Registry 不提供动态加载能力。
    pub fn with_tool_registry(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    /// Limits messages sent to the model at each request boundary. System messages are
    /// always retained, and tool-call exchanges are never split.
    pub fn with_history_message_budget(mut self, max_messages: usize) -> Self {
        self.context_manager = ContextManager::new(max_messages);
        self
    }

    pub fn enqueue_user(&mut self, content: impl Into<String>) {
        self.pending_inputs.push_back(content.into());
    }
    pub fn conversation(&self) -> &Conversation {
        &self.conversation
    }
    #[allow(dead_code)]
    pub fn pending_input_count(&self) -> usize {
        self.pending_inputs.len()
    }
    #[allow(dead_code)]
    pub fn state(&self) -> AgentState {
        self.state
    }

    #[allow(dead_code)]
    pub fn session_data(
        &self,
    ) -> (
        Vec<Message>,
        Vec<String>,
        usize,
        usize,
        RetryPolicy,
        usize,
        Vec<RunReport>,
    ) {
        (
            self.conversation.messages().to_vec(),
            self.pending_inputs.iter().cloned().collect(),
            self.max_steps,
            self.steps_taken,
            self.retry_policy.clone(),
            self.context_manager.max_messages(),
            self.run_reports.clone(),
        )
    }

    #[allow(clippy::too_many_arguments, dead_code)]
    pub fn from_session_data(
        model: M,
        messages: Vec<Message>,
        pending_inputs: Vec<String>,
        max_steps: usize,
        steps_taken: usize,
        retry_policy: RetryPolicy,
        history_message_budget: usize,
        run_reports: Vec<RunReport>,
    ) -> Result<Self, AgentError> {
        if max_steps == 0 {
            return Err(AgentError::InvalidMaxSteps);
        }
        Ok(Self {
            model,
            conversation: Conversation::from_messages(messages),
            tools: ToolRegistry::new(),
            pending_inputs: pending_inputs.into(),
            max_steps,
            steps_taken,
            retry_policy,
            context_manager: ContextManager::new(history_message_budget),
            state: AgentState::Ready,
            run_reports,
        })
    }

    /// 使用一个永不取消的令牌运行。
    pub async fn run(&mut self, events: mpsc::Sender<AgentEvent>) -> Result<RunReport, AgentError> {
        self.run_with_cancellation(events, &CancellationToken::new())
            .await
    }

    /// 在模型等待、流转发、退避，以及每个工具开始前协作式响应取消。
    pub async fn run_with_cancellation(
        &mut self,
        events: mpsc::Sender<AgentEvent>,
        cancellation: &CancellationToken,
    ) -> Result<RunReport, AgentError> {
        self.state = AgentState::Running;
        match self
            .send_event(&events, AgentEvent::Started, cancellation)
            .await
        {
            Ok(()) => {}
            Err(ModelCallOutcome::Cancelled) => return self.cancel(events, 0, 0, 0),
            Err(ModelCallOutcome::Error(error)) => return self.fail(events, error).await,
        }
        let mut needs_model_turn = false;
        let mut tool_calls_processed = 0;
        let mut model_attempts = 0;
        let mut retries = 0;

        while self.steps_taken < self.max_steps {
            if cancellation.is_cancelled() {
                return self.cancel(events, tool_calls_processed, model_attempts, retries);
            }
            if !needs_model_turn {
                let Some(input) = self.pending_inputs.pop_front() else {
                    return self
                        .finish(
                            events,
                            TerminationReason::NoPendingInput,
                            tool_calls_processed,
                            model_attempts,
                            retries,
                        )
                        .await;
                };
                self.conversation.add_user(input.clone());
                if let Err(outcome) = self
                    .send_event(&events, AgentEvent::Input { content: input }, cancellation)
                    .await
                {
                    return self
                        .handle_outcome(
                            events,
                            outcome,
                            tool_calls_processed,
                            model_attempts,
                            retries,
                        )
                        .await;
                }
            }
            needs_model_turn = false;

            let response = match self
                .call_model_with_retry(&events, cancellation, &mut model_attempts, &mut retries)
                .await
            {
                Ok(response) => response,
                Err(outcome) => {
                    return self
                        .handle_outcome(
                            events,
                            outcome,
                            tool_calls_processed,
                            model_attempts,
                            retries,
                        )
                        .await;
                }
            };
            self.steps_taken += 1;
            self.conversation
                .add_assistant_response(response.content.clone(), response.tool_calls.clone());
            if let Err(outcome) = self
                .send_event(
                    &events,
                    AgentEvent::AssistantRecorded {
                        response: response.clone(),
                    },
                    cancellation,
                )
                .await
            {
                return self
                    .handle_outcome(
                        events,
                        outcome,
                        tool_calls_processed,
                        model_attempts,
                        retries,
                    )
                    .await;
            }

            if !response.tool_calls.is_empty() {
                for call in response.tool_calls {
                    if cancellation.is_cancelled() {
                        return self.cancel(events, tool_calls_processed, model_attempts, retries);
                    }
                    if let Err(outcome) = self
                        .send_event(
                            &events,
                            AgentEvent::ToolStarted {
                                call_id: call.id.clone(),
                                name: call.name.clone(),
                            },
                            cancellation,
                        )
                        .await
                    {
                        return self
                            .handle_outcome(
                                events,
                                outcome,
                                tool_calls_processed,
                                model_attempts,
                                retries,
                            )
                            .await;
                    }
                    // 同步工具一旦开始不能被本章的协作式令牌抢占；取消会阻止下一项工具。
                    let content =
                        tool_result_content(self.tools.execute(&call.name, &call.arguments));
                    self.conversation.add_tool(call.id.clone(), content);
                    tool_calls_processed += 1;
                    if let Err(outcome) = self
                        .send_event(
                            &events,
                            AgentEvent::ToolFinished {
                                call_id: call.id,
                                name: call.name,
                            },
                            cancellation,
                        )
                        .await
                    {
                        return self
                            .handle_outcome(
                                events,
                                outcome,
                                tool_calls_processed,
                                model_attempts,
                                retries,
                            )
                            .await;
                    }
                }
                needs_model_turn = true;
            } else if self.pending_inputs.is_empty() {
                return self
                    .finish(
                        events,
                        TerminationReason::NoPendingInput,
                        tool_calls_processed,
                        model_attempts,
                        retries,
                    )
                    .await;
            }
        }

        self.finish(
            events,
            TerminationReason::MaxStepsReached,
            tool_calls_processed,
            model_attempts,
            retries,
        )
        .await
    }

    async fn call_model_with_retry(
        &mut self,
        events: &mpsc::Sender<AgentEvent>,
        cancellation: &CancellationToken,
        model_attempts: &mut usize,
        retries: &mut usize,
    ) -> Result<ChatResponse, ModelCallOutcome> {
        loop {
            if cancellation.is_cancelled() {
                return Err(ModelCallOutcome::Cancelled);
            }
            *model_attempts += 1;
            self.send_event(
                events,
                AgentEvent::Attempt {
                    attempt: *model_attempts,
                },
                cancellation,
            )
            .await?;
            match self.call_model_once(events, cancellation).await {
                Ok(response) => return Ok(response),
                Err(ModelCallOutcome::Error(AgentError::Model(error)))
                    if error.is_retryable() && *retries < self.retry_policy.max_retries =>
                {
                    *retries += 1;
                    let delay = self.retry_policy.backoff_for_retry(*retries);
                    self.send_event(
                        events,
                        AgentEvent::Retry {
                            attempt: *retries + 1,
                            delay,
                        },
                        cancellation,
                    )
                    .await?;
                    tokio::select! {
                        _ = cancellation.cancelled() => return Err(ModelCallOutcome::Cancelled),
                        _ = tokio::time::sleep(delay) => {}
                    }
                }
                Err(ModelCallOutcome::Error(AgentError::Model(error))) if error.is_retryable() => {
                    return Err(ModelCallOutcome::Error(AgentError::RetriesExhausted {
                        attempts: *model_attempts,
                        source: error,
                    }));
                }
                Err(outcome) => return Err(outcome),
            }
        }
    }

    async fn call_model_once(
        &mut self,
        events: &mpsc::Sender<AgentEvent>,
        cancellation: &CancellationToken,
    ) -> Result<ChatResponse, ModelCallOutcome> {
        let context = self.context_manager.window(self.conversation.messages());
        if context.removed_messages > 0 {
            self.send_event(
                events,
                AgentEvent::ContextTrimmed {
                    removed_messages: context.removed_messages,
                },
                cancellation,
            )
            .await?;
        }
        let history = context.messages;
        let definitions = self.tools.definitions();
        let (model_events, mut model_rx) = mpsc::channel(32);
        let producer = tokio::spawn({
            let model = self.model.clone();
            async move {
                model
                    .chat_stream(&history, &definitions, model_events)
                    .await
            }
        });

        loop {
            tokio::select! {
                _ = cancellation.cancelled() => {
                    producer.abort();
                    return Err(ModelCallOutcome::Cancelled);
                }
                event = model_rx.recv() => match event {
                    Some(event) => match self.send_event(events, AgentEvent::Model(event), cancellation).await {
                        Ok(()) => {}
                        Err(outcome) => {
                            producer.abort();
                            return Err(outcome);
                        }
                    },
                    None => break,
                }
            }
        }
        producer
            .await
            .map_err(|error| ModelCallOutcome::Error(AgentError::ModelTask(error.to_string())))?
            .map_err(|error| ModelCallOutcome::Error(AgentError::Model(error)))
    }

    async fn send_event(
        &self,
        events: &mpsc::Sender<AgentEvent>,
        event: AgentEvent,
        cancellation: &CancellationToken,
    ) -> Result<(), ModelCallOutcome> {
        if events.is_closed() {
            return Err(ModelCallOutcome::Error(AgentError::EventReceiverClosed));
        }
        tokio::select! {
            _ = cancellation.cancelled() => Err(ModelCallOutcome::Cancelled),
            result = events.send(event) => result
                .map_err(|_| ModelCallOutcome::Error(AgentError::EventReceiverClosed)),
        }
    }

    async fn handle_outcome(
        &mut self,
        events: mpsc::Sender<AgentEvent>,
        outcome: ModelCallOutcome,
        tool_calls_processed: usize,
        model_attempts: usize,
        retries: usize,
    ) -> Result<RunReport, AgentError> {
        match outcome {
            ModelCallOutcome::Cancelled => {
                self.cancel(events, tool_calls_processed, model_attempts, retries)
            }
            ModelCallOutcome::Error(error) => self.fail(events, error).await,
        }
    }

    fn cancel(
        &mut self,
        events: mpsc::Sender<AgentEvent>,
        tool_calls_processed: usize,
        model_attempts: usize,
        retries: usize,
    ) -> Result<RunReport, AgentError> {
        if !events.is_closed() {
            let _ = events.try_send(AgentEvent::Cancelled);
        }
        Ok(self.stop(
            TerminationReason::Cancelled,
            tool_calls_processed,
            model_attempts,
            retries,
        ))
    }

    async fn finish(
        &mut self,
        events: mpsc::Sender<AgentEvent>,
        termination: TerminationReason,
        tool_calls_processed: usize,
        model_attempts: usize,
        retries: usize,
    ) -> Result<RunReport, AgentError> {
        let report = self.stop(termination, tool_calls_processed, model_attempts, retries);
        match events.send(AgentEvent::Finished(report.clone())).await {
            Ok(()) => Ok(report),
            Err(_) => self.fail(events, AgentError::EventReceiverClosed).await,
        }
    }

    async fn fail(
        &mut self,
        events: mpsc::Sender<AgentEvent>,
        error: AgentError,
    ) -> Result<RunReport, AgentError> {
        self.state = AgentState::Failed;
        if !events.is_closed() {
            let _ = events
                .send(AgentEvent::Failed {
                    error: error.to_string(),
                })
                .await;
        }
        Err(error)
    }

    fn stop(
        &mut self,
        termination: TerminationReason,
        tool_calls_processed: usize,
        model_attempts: usize,
        retries: usize,
    ) -> RunReport {
        self.state = AgentState::Stopped(termination);
        let report = RunReport {
            steps_taken: self.steps_taken,
            termination,
            tool_calls_processed,
            model_attempts,
            retries,
        };
        self.run_reports.push(report.clone());
        report
    }
}

enum ModelCallOutcome {
    Cancelled,
    Error(AgentError),
}

fn tool_result_content(result: Result<serde_json::Value, ToolError>) -> String {
    match result {
        Ok(value) => json!({ "ok": true, "result": value }).to_string(),
        Err(error) => json!({
            "ok": false,
            "error": { "code": error.code(), "message": error.to_string() }
        })
        .to_string(),
    }
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
