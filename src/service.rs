use crate::agent::{AgentEvent, CancellationToken};
use tokio::sync::mpsc;

/// Local actor-facing session control. Transport adapters may consume events later, but this
/// module intentionally opens no socket and exposes no public network server.
#[derive(Clone)]
pub struct SessionClient {
    sender: mpsc::Sender<SessionCommand>,
    pub cancellation: CancellationToken,
}
pub struct SessionActor {
    receiver: mpsc::Receiver<SessionCommand>,
    events: mpsc::Sender<AgentEvent>,
}
#[derive(Debug)]
pub enum SessionCommand {
    Prompt(String),
    Cancel,
}
impl SessionActor {
    pub fn local(capacity: usize, events: mpsc::Sender<AgentEvent>) -> (Self, SessionClient) {
        let (sender, receiver) = mpsc::channel(capacity.max(1));
        let cancellation = CancellationToken::new();
        (
            Self { receiver, events },
            SessionClient {
                sender,
                cancellation,
            },
        )
    }
    /// Processes one local command. The host owns Agent execution; prompts/events stay channel-bound.
    pub async fn next(&mut self) -> Option<SessionCommand> {
        self.receiver.recv().await
    }
    pub async fn forward(
        &self,
        event: AgentEvent,
    ) -> Result<(), mpsc::error::SendError<AgentEvent>> {
        self.events.send(event).await
    }
}
impl SessionClient {
    pub async fn prompt(
        &self,
        text: impl Into<String>,
    ) -> Result<(), mpsc::error::SendError<SessionCommand>> {
        self.sender.send(SessionCommand::Prompt(text.into())).await
    }
    pub async fn cancel(&self) -> Result<(), mpsc::error::SendError<SessionCommand>> {
        self.cancellation.cancel();
        self.sender.send(SessionCommand::Cancel).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn local_actor_preserves_commands_and_forwards_existing_events() {
        let (events_tx, mut events_rx) = mpsc::channel(1);
        let (mut actor, client) = SessionActor::local(2, events_tx);
        client.prompt("hello").await.unwrap();
        assert!(matches!(actor.next().await,Some(SessionCommand::Prompt(text)) if text=="hello"));
        actor.forward(AgentEvent::Started).await.unwrap();
        assert!(matches!(events_rx.recv().await, Some(AgentEvent::Started)));
        client.cancel().await.unwrap();
        assert!(client.cancellation.is_cancelled());
        assert!(matches!(actor.next().await, Some(SessionCommand::Cancel)));
    }
}
