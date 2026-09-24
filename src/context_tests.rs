use super::ContextManager;
use crate::{llm::ToolCall, message::Message};

fn tool_call(id: &str) -> ToolCall {
    ToolCall {
        id: id.into(),
        name: "get_weather".into(),
        arguments: "{}".into(),
    }
}

#[test]
fn retains_system_messages_and_the_newest_messages_within_the_budget() {
    let history = vec![
        Message::system("instructions"),
        Message::user("old question"),
        Message::assistant("old answer"),
        Message::user("new question"),
    ];

    let window = ContextManager::new(3).window(&history);

    assert_eq!(window.removed_messages, 1);
    assert_eq!(
        window.messages,
        vec![
            Message::system("instructions"),
            Message::assistant("old answer"),
            Message::user("new question"),
        ]
    );
}

#[test]
fn never_splits_an_assistant_tool_call_and_its_results() {
    let call = tool_call("call_1");
    let history = vec![
        Message::system("instructions"),
        Message::user("old question"),
        Message::assistant_with_tool_calls(None, vec![call.clone()]),
        Message::tool("call_1", "old result"),
        Message::user("new question"),
    ];

    let window = ContextManager::new(3).window(&history);

    assert_eq!(window.removed_messages, 3);
    assert_eq!(
        window.messages,
        vec![
            Message::system("instructions"),
            Message::user("new question")
        ]
    );
}

#[test]
fn preserves_the_complete_recent_tool_turn_when_it_fits() {
    let call = tool_call("call_1");
    let history = vec![
        Message::system("instructions"),
        Message::user("old question"),
        Message::assistant_with_tool_calls(None, vec![call.clone()]),
        Message::tool("call_1", "result"),
    ];

    let window = ContextManager::new(3).window(&history);

    assert_eq!(window.removed_messages, 1);
    assert_eq!(
        window.messages,
        vec![
            Message::system("instructions"),
            Message::assistant_with_tool_calls(None, vec![call]),
            Message::tool("call_1", "result"),
        ]
    );
}

#[test]
fn system_messages_take_priority_when_they_exceed_the_budget() {
    let history = vec![
        Message::system("first"),
        Message::system("second"),
        Message::user("question"),
    ];

    let window = ContextManager::new(1).window(&history);

    assert_eq!(window.removed_messages, 1);
    assert_eq!(
        window.messages,
        vec![Message::system("first"), Message::system("second")]
    );
}
