use crate::{
    agent::StreamChatModel,
    llm::{ChatResponse, LlmError, StreamEvent, ToolDefinition},
    message::Message,
};
use std::future::Future;
use tokio::sync::mpsc;
/// A minimal model-call boundary. It can inspect a request but does not alter agent state.
pub trait ModelMiddleware: Clone + Send + Sync + 'static {
    fn before_model(&self, messages: &[Message], tools: &[ToolDefinition]) -> Result<(), LlmError>;
}
#[derive(Clone)]
pub struct MiddlewareModel<M, W> {
    model: M,
    middleware: W,
}
impl<M, W> MiddlewareModel<M, W> {
    pub fn new(model: M, middleware: W) -> Self {
        Self { model, middleware }
    }
}
impl<M: StreamChatModel, W: ModelMiddleware> StreamChatModel for MiddlewareModel<M, W> {
    fn chat_stream(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        tx: mpsc::Sender<StreamEvent>,
    ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send {
        let gate = self.middleware.before_model(messages, tools);
        let model = self.model.clone();
        let messages = messages.to_vec();
        let tools = tools.to_vec();
        async move {
            gate?;
            model.chat_stream(&messages, &tools, tx).await
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        llm::{FinishReason, Usage},
        message::Message,
    };
    use std::{
        future::ready,
        sync::{Arc, Mutex},
    };
    #[derive(Clone)]
    struct Fake;
    impl StreamChatModel for Fake {
        fn chat_stream(
            &self,
            _: &[Message],
            _: &[ToolDefinition],
            _: mpsc::Sender<StreamEvent>,
        ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send {
            ready(Ok(ChatResponse {
                content: String::new(),
                tool_calls: vec![],
                finish_reason: FinishReason::Stop,
                usage: Usage::default(),
            }))
        }
    }
    #[derive(Clone)]
    struct Gate(Arc<Mutex<usize>>);
    impl ModelMiddleware for Gate {
        fn before_model(&self, _: &[Message], _: &[ToolDefinition]) -> Result<(), LlmError> {
            *self.0.lock().unwrap() += 1;
            Ok(())
        }
    }
    #[tokio::test]
    async fn wraps_the_model_boundary() {
        let count = Arc::new(Mutex::new(0));
        let model = MiddlewareModel::new(Fake, Gate(count.clone()));
        let (tx, _) = mpsc::channel(1);
        model
            .chat_stream(&[Message::user("x")], &[], tx)
            .await
            .unwrap();
        assert_eq!(*count.lock().unwrap(), 1);
    }
}
