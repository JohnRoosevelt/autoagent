use crate::{
    llm::{ChatResponse, LlmError, StreamEvent},
    message::{Conversation, Message},
};
use std::{collections::VecDeque, future::Future};
use tokio::sync::mpsc;

/// Agent 依赖的最小模型能力；实现不需要暴露 SSE 协议细节。
pub trait StreamChatModel: Clone + Send + Sync + 'static {
    fn chat_stream(
        &self,
        messages: &[Message],
        tx: mpsc::Sender<StreamEvent>,
    ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send;
}

/// 一次 Agent 运行结束的明确原因。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminationReason {
    /// 所有待处理用户输入均已完成，当前没有后续工作。
    NoPendingInput,
    /// 为避免未来工具分支意外循环，本次运行达到步数上限。
    MaxStepsReached,
}

/// Agent 当前的生命周期状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentState {
    Ready,
    Running,
    Stopped(TerminationReason),
    Failed,
}

/// 一次运行的摘要，供调用方记录或显示。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunReport {
    pub steps_taken: usize,
    pub termination: TerminationReason,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent 配置错误: max_steps 必须大于 0")]
    InvalidMaxSteps,

    #[error(transparent)]
    Model(#[from] LlmError),

    #[error("Agent 事件接收方已关闭")]
    EventReceiverClosed,

    #[error("Agent 模型任务失败: {0}")]
    ModelTask(String),
}

/// 管理一次运行的会话、待处理输入和循环状态。
///
/// 本章每个 assistant 回复都会结束当前一步；后续工具调用会在回复回填后，
/// 以“执行工具并追加结果，然后继续”的分支扩展到此循环中。
pub struct Agent<M> {
    model: M,
    conversation: Conversation,
    pending_inputs: VecDeque<String>,
    max_steps: usize,
    steps_taken: usize,
    state: AgentState,
}

impl<M: StreamChatModel> Agent<M> {
    pub fn new(model: M, conversation: Conversation, max_steps: usize) -> Result<Self, AgentError> {
        if max_steps == 0 {
            return Err(AgentError::InvalidMaxSteps);
        }

        Ok(Self {
            model,
            conversation,
            pending_inputs: VecDeque::new(),
            max_steps,
            steps_taken: 0,
            state: AgentState::Ready,
        })
    }

    /// 排入下一次模型调用的用户输入。输入只会在实际执行该步时写入历史。
    pub fn enqueue_user(&mut self, content: impl Into<String>) {
        self.pending_inputs.push_back(content.into());
    }

    #[allow(dead_code)]
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

    /// 运行直到没有待处理输入，或到达 `max_steps`。
    ///
    /// Agent 消费每一轮模型的内部事件流，再将原样的领域事件转发给显示层。
    pub async fn run(
        &mut self,
        events: mpsc::Sender<StreamEvent>,
    ) -> Result<RunReport, AgentError> {
        self.state = AgentState::Running;

        while self.steps_taken < self.max_steps {
            let Some(input) = self.pending_inputs.pop_front() else {
                return Ok(self.stop(TerminationReason::NoPendingInput));
            };

            self.conversation.add_user(input);
            let history = self.conversation.messages().to_vec();
            let (model_events, mut model_rx) = mpsc::channel(32);
            let producer = tokio::spawn({
                let model = self.model.clone();
                async move { model.chat_stream(&history, model_events).await }
            });

            while let Some(event) = model_rx.recv().await {
                if events.send(event).await.is_err() {
                    producer.abort();
                    self.state = AgentState::Failed;
                    return Err(AgentError::EventReceiverClosed);
                }
            }

            let response = producer
                .await
                .map_err(|error| AgentError::ModelTask(error.to_string()))??;
            self.conversation.add_assistant(response.content);
            self.steps_taken += 1;
        }

        Ok(self.stop(if self.pending_inputs.is_empty() {
            TerminationReason::NoPendingInput
        } else {
            TerminationReason::MaxStepsReached
        }))
    }

    fn stop(&mut self, termination: TerminationReason) -> RunReport {
        self.state = AgentState::Stopped(termination);
        RunReport {
            steps_taken: self.steps_taken,
            termination,
        }
    }
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
