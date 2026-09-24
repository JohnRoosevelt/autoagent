use super::{Conversation, Message};
use crate::llm::ToolCall;

#[test]
fn serializes_messages_with_openai_compatible_roles() {
    assert_eq!(
        serde_json::to_value(Message::system("回答要简洁")).unwrap(),
        serde_json::json!({"role":"system","content":"回答要简洁"})
    );
}

#[test]
fn serializes_tool_results_with_the_call_id() {
    assert_eq!(
        serde_json::to_value(Message::tool("call_1", "{\"ok\":true}")).unwrap(),
        serde_json::json!({"role":"tool","tool_call_id":"call_1","content":"{\"ok\":true}"})
    );

    let mut conversation = Conversation::new();
    conversation.add_tool("call_2", "result");
    assert_eq!(conversation.messages(), [Message::tool("call_2", "result")]);
}

#[test]
fn assistant_tool_calls_are_preserved_for_the_next_request() {
    let calls = vec![ToolCall {
        id: "call_1".into(),
        name: "weather".into(),
        arguments: "{\"city\":\"北京\"}".into(),
    }];
    let mut conversation = Conversation::new();
    conversation.add_assistant_response(String::new(), calls.clone());
    assert_eq!(
        conversation.messages()[0],
        Message::assistant_with_tool_calls(None, calls)
    );
    assert_eq!(
        serde_json::to_value(&conversation.messages()[0]).unwrap(),
        serde_json::json!({"role":"assistant","content":null,"tool_calls":[{"id":"call_1","type":"function","function":{"name":"weather","arguments":"{\"city\":\"北京\"}"}}]})
    );
}
