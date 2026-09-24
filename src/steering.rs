use crate::agent::CancellationToken;
use tokio::sync::mpsc;

/// A bounded, local control plane for a running agent session.
/// Queued text is deliberately applied only when the host reaches a turn boundary;
/// cancellation is cooperative and shares the Agent's existing cancellation token.
pub struct SteeringInbox {
    receiver: mpsc::Receiver<SteeringCommand>,
    cancellation: CancellationToken,
}

#[derive(Clone)]
pub struct SteeringHandle {
    sender: mpsc::Sender<SteeringCommand>,
    cancellation: CancellationToken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SteeringCommand {
    Enqueue(String),
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SteeringError {
    #[error("steering inbox is full")]
    Full,
    #[error("steering inbox is closed")]
    Closed,
    #[error("steering text must not be empty")]
    Empty,
}

impl SteeringInbox {
    pub fn new(capacity: usize, cancellation: CancellationToken) -> (Self, SteeringHandle) {
        let (sender, receiver) = mpsc::channel(capacity.max(1));
        (
            Self {
                receiver,
                cancellation: cancellation.clone(),
            },
            SteeringHandle {
                sender,
                cancellation,
            },
        )
    }

    /// Drain commands at a host-defined safe point, preserving submission order.
    pub fn drain_turn_boundary(&mut self) -> Vec<String> {
        let mut queued = Vec::new();
        while let Ok(command) = self.receiver.try_recv() {
            match command {
                SteeringCommand::Enqueue(text) => queued.push(text),
                SteeringCommand::Cancel => self.cancellation.cancel(),
            }
        }
        queued
    }
}

impl SteeringHandle {
    pub fn steer(&self, text: impl Into<String>) -> Result<(), SteeringError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(SteeringError::Empty);
        }
        self.send(SteeringCommand::Enqueue(text))
    }

    pub fn cancel(&self) -> Result<(), SteeringError> {
        // Mark first so a full inbox cannot delay an urgent cancellation.
        self.cancellation.cancel();
        self.send(SteeringCommand::Cancel)
    }

    fn send(&self, command: SteeringCommand) -> Result<(), SteeringError> {
        self.sender.try_send(command).map_err(|error| match error {
            mpsc::error::TrySendError::Full(_) => SteeringError::Full,
            mpsc::error::TrySendError::Closed(_) => SteeringError::Closed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queues_steering_in_order_until_a_turn_boundary() {
        let cancellation = CancellationToken::new();
        let (mut inbox, handle) = SteeringInbox::new(2, cancellation);
        handle.steer("use concise output").unwrap();
        handle.steer("then summarize").unwrap();
        assert_eq!(
            inbox.drain_turn_boundary(),
            ["use concise output", "then summarize"]
        );
        assert!(inbox.drain_turn_boundary().is_empty());
    }

    #[test]
    fn cancellation_is_cooperative_even_when_the_inbox_is_full() {
        let cancellation = CancellationToken::new();
        let (_inbox, handle) = SteeringInbox::new(1, cancellation.clone());
        handle.steer("queued").unwrap();
        assert_eq!(handle.cancel(), Err(SteeringError::Full));
        assert!(cancellation.is_cancelled());
    }
}
