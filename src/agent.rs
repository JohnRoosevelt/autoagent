use crate::{
    llm::{ChatResponse, LlmError, StreamEvent, ToolCall, ToolDefinition},
    message::{Conversation, Message},
};
use std::{collections::VecDeque, future::Future};
use tokio::sync::mpsc;

/// Agent 依赖的最小模型能力；实现不需要暴露 SSE 协议细节。
pub trait StreamChatModel: Clone + Send + Sync + 'static {
    fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        tx: mpsc::Sender<StreamEvent>,
    ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send;
}

/// 一次 Agent 运行结束的明确原因。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminationReason {
    NoPendingInput,
    MaxStepsReached,
    /// assistant 工具调用已入账，等待宿主在第 06 章执行并追加 tool result。
    ToolCallsRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentState {
    Ready,
    Running,
    Stopped(TerminationReason),
    Failed,
}

/// 一次运行的摘要，供调用方记录、显示或接续工具执行。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunReport {
    pub steps_taken: usize,
    pub termination: TerminationReason,
    pub requested_tool_calls: Vec<ToolCall>,
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

/// 管理一次运行的会话、工具声明、待处理输入和循环状态。
pub struct Agent<M> {
    model: M,
    conversation: Conversation,
    tools: Vec<ToolDefinition>,
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
            tools: Vec::new(),
            pending_inputs: VecDeque::new(),
            max_steps,
            steps_taken: 0,
            state: AgentState::Ready,
        })
    }

    /// 为本次运行声明可供模型选择的函数；不代表函数已经实现或可执行。
    #[allow(dead_code)]
    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = tools;
        self
    }
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

    pub async fn run(
        &mut self,
        events: mpsc::Sender<StreamEvent>,
    ) -> Result<RunReport, AgentError> {
        self.state = AgentState::Running;
        while self.steps_taken < self.max_steps {
            let Some(input) = self.pending_inputs.pop_front() else {
                return Ok(self.stop(TerminationReason::NoPendingInput, Vec::new()));
            };
            self.conversation.add_user(input);
            let history = self.conversation.messages().to_vec();
            let (model_events, mut model_rx) = mpsc::channel(32);
            let producer = tokio::spawn({
                let model = self.model.clone();
                let tools = self.tools.clone();
                async move { model.chat_stream(&history, &tools, model_events).await }
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
            self.conversation
                .add_assistant_response(response.content, response.tool_calls.clone());
            self.steps_taken += 1;
            if !response.tool_calls.is_empty() {
                return Ok(self.stop(TerminationReason::ToolCallsRequested, response.tool_calls));
            }
        }
        Ok(self.stop(
            if self.pending_inputs.is_empty() {
                TerminationReason::NoPendingInput
            } else {
                TerminationReason::MaxStepsReached
            },
            Vec::new(),
        ))
    }

    fn stop(
        &mut self,
        termination: TerminationReason,
        requested_tool_calls: Vec<ToolCall>,
    ) -> RunReport {
        self.state = AgentState::Stopped(termination);
        RunReport {
            steps_taken: self.steps_taken,
            termination,
            requested_tool_calls,
        }
    }
}

#[cfg(test)]
#[path = "agent_tests.rs"]
mod tests;
