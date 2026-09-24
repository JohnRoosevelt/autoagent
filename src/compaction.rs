use crate::message::{Message, Role};

/// Produces a compact request view; the caller owns summarization and the original ledger remains intact.
pub struct Compactor;

impl Compactor {
    pub fn compact(
        messages: &[Message],
        keep_recent: usize,
        summary: impl Into<String>,
    ) -> Vec<Message> {
        if messages.len() <= keep_recent {
            return messages.to_vec();
        }
        let split_at = messages.len() - keep_recent;
        let mut compacted = vec![Message::new(
            Role::System,
            format!("Conversation summary: {}", summary.into()),
        )];
        compacted.extend_from_slice(&messages[split_at..]);
        compacted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replaces_old_history_with_caller_supplied_summary() {
        let messages = vec![
            Message::new(Role::User, "old task"),
            Message::new(Role::Assistant, "old answer"),
            Message::new(Role::User, "current task"),
        ];
        let compacted = Compactor::compact(&messages, 1, "task is in progress");
        assert_eq!(compacted.len(), 2);
        assert_eq!(compacted[0].role, Role::System);
        assert_eq!(compacted[1].content.as_deref(), Some("current task"));
        assert_eq!(messages.len(), 3);
    }

    #[test]
    fn leaves_short_history_unchanged() {
        let messages = vec![Message::new(Role::User, "task")];
        assert_eq!(Compactor::compact(&messages, 1, "unused"), messages);
    }
}
