use crate::message::{Message, Role};

/// Produces a bounded model-request history without mutating the conversation ledger.
///
/// The budget counts messages. System messages are always retained; if they alone exceed
/// the configured budget, preserving them takes precedence. An assistant tool-call message
/// and its immediately following tool results are retained or removed together.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextManager {
    max_messages: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextWindow {
    pub messages: Vec<Message>,
    pub removed_messages: usize,
}

impl ContextManager {
    pub fn new(max_messages: usize) -> Self {
        Self { max_messages }
    }

    pub fn window(&self, history: &[Message]) -> ContextWindow {
        let system_count = history
            .iter()
            .filter(|message| message.role == Role::System)
            .count();
        let mut keep = vec![false; history.len()];
        for (index, message) in history.iter().enumerate() {
            if message.role == Role::System {
                keep[index] = true;
            }
        }

        let mut remaining = self.max_messages.saturating_sub(system_count);
        for unit in conversation_units(history).into_iter().rev() {
            let length = unit.end - unit.start;
            if length > remaining {
                break;
            }
            keep[unit.start..unit.end].fill(true);
            remaining -= length;
        }

        let messages: Vec<_> = history
            .iter()
            .zip(keep)
            .filter(|(_, retained)| *retained)
            .map(|(message, _)| message.clone())
            .collect();
        ContextWindow {
            removed_messages: history.len() - messages.len(),
            messages,
        }
    }
}

#[derive(Clone, Copy)]
struct ConversationUnit {
    start: usize,
    end: usize,
}

fn conversation_units(history: &[Message]) -> Vec<ConversationUnit> {
    let mut units = Vec::new();
    let mut index = 0;
    while index < history.len() {
        if history[index].role == Role::System {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        if history[start].role == Role::Assistant && !history[start].tool_calls.is_empty() {
            while index < history.len() && history[index].role == Role::Tool {
                index += 1;
            }
        }
        units.push(ConversationUnit { start, end: index });
    }
    units
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
