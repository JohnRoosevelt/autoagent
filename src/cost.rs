use crate::{llm::Usage, message::Message};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StablePrefix {
    pub fingerprint: String,
    pub message_count: usize,
}
impl StablePrefix {
    /// Fingerprints the caller-selected stable prefix without sending provider cache directives.
    pub fn from_messages(messages: &[Message]) -> Self {
        let mut hash = 0xcbf29ce484222325u64;
        for message in messages {
            for byte in format!("{:?}|{:?}|", message.role, message.content).bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        Self {
            fingerprint: format!("{hash:016x}"),
            message_count: messages.len(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TokenCost {
    pub input_tokens: u64,
    pub cached_input_tokens: u64,
    pub output_tokens: u64,
    pub micros: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Pricing {
    pub input_micros_per_token: u64,
    pub cached_input_micros_per_token: u64,
    pub output_micros_per_token: u64,
}
impl TokenCost {
    pub fn from_usage(usage: Usage, cached_input_tokens: u64, pricing: Pricing) -> Self {
        let input = usage.input_tokens as u64;
        let output = usage.output_tokens as u64;
        let cached = cached_input_tokens.min(input);
        Self {
            input_tokens: input,
            cached_input_tokens: cached,
            output_tokens: output,
            micros: (input - cached)
                .saturating_mul(pricing.input_micros_per_token)
                .saturating_add(cached.saturating_mul(pricing.cached_input_micros_per_token))
                .saturating_add(output.saturating_mul(pricing.output_micros_per_token)),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    #[test]
    fn stable_prefix_and_cost_are_deterministic() {
        let prefix = StablePrefix::from_messages(&[Message::system("rules")]);
        assert_eq!(
            prefix,
            StablePrefix::from_messages(&[Message::system("rules")])
        );
        let cost = TokenCost::from_usage(
            Usage {
                input_tokens: 10,
                output_tokens: 2,
            },
            4,
            Pricing {
                input_micros_per_token: 3,
                cached_input_micros_per_token: 1,
                output_micros_per_token: 5,
            },
        );
        assert_eq!(cost.micros, 32);
    }
}
