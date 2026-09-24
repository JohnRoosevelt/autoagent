use crate::{
    llm::{ChatResponse, LlmError, StreamEvent, ToolDefinition},
    message::{Conversation, Message},
    tool::{ToolError, ToolRegistry},
};
use serde_json::json;
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
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentState {
    Ready,
    Running,
    Stopped(TerminationReason),
    Failed,
}

/// 一次运行的摘要，供调用方记录和显示。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunReport {
    /// 模型调用数；工具执行不单独计为 Agent step。
    pub steps_taken: usize,
    pub termination: TerminationReason,
    /// 本次已被 Registry 串行处理（成功或失败）的 tool call 数量。
    pub tool_calls_processed: usize,
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

/// 管理一次运行的会话、运行时本地工具、待处理输入和循环状态。
pub struct Agent<M> {
    model: M,
    conversation: Conversation,
    tools: ToolRegistry,
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
            tools: ToolRegistry::new(),
            pending_inputs: VecDeque::new(),
            max_steps,
            steps_taken: 0,
            state: AgentState::Ready,
        })
    }

    /// 注入启动时组装的本地 Registry；该 Registry 不提供动态加载能力。
    pub fn with_tool_registry(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
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

    pub async fn run(
        &mut self,
        events: mpsc::Sender<StreamEvent>,
    ) -> Result<RunReport, AgentError> {
        self.state = AgentState::Running;
        let mut needs_model_turn = false;
        let mut tool_calls_processed = 0;

        while self.steps_taken < self.max_steps {
            if !needs_model_turn {
                let Some(input) = self.pending_inputs.pop_front() else {
                    return Ok(self.stop(TerminationReason::NoPendingInput, tool_calls_processed));
                };
                self.conversation.add_user(input);
            }
            needs_model_turn = false;

            let history = self.conversation.messages().to_vec();
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
            self.steps_taken += 1;
            self.conversation
                .add_assistant_response(response.content, response.tool_calls.clone());

            if !response.tool_calls.is_empty() {
                for call in response.tool_calls {
                    let content =
                        tool_result_content(self.tools.execute(&call.name, &call.arguments));
                    self.conversation.add_tool(call.id, content);
                    tool_calls_processed += 1;
                }
                // The adjacent assistant/tool messages are now part of history for the next call.
                needs_model_turn = true;
            } else if self.pending_inputs.is_empty() {
                return Ok(self.stop(TerminationReason::NoPendingInput, tool_calls_processed));
            }
        }

        Ok(self.stop(TerminationReason::MaxStepsReached, tool_calls_processed))
    }

    fn stop(&mut self, termination: TerminationReason, tool_calls_processed: usize) -> RunReport {
        self.state = AgentState::Stopped(termination);
        RunReport {
            steps_taken: self.steps_taken,
            termination,
            tool_calls_processed,
        }
    }
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
