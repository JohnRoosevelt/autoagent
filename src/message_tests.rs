use super::{Conversation, Message, Role};

#[test]
fn serializes_messages_with_openai_compatible_roles() {
    let message = Message::system("回答要简洁");

    assert_eq!(
        serde_json::to_value(message).unwrap(),
        serde_json::json!({"role": "system", "content": "回答要简洁"})
    );
}

#[test]
fn conversation_preserves_messages_in_append_order() {
    let mut conversation = Conversation::new();
    conversation.add_system("你是一个助手。");
    conversation.add_user("什么是 HTTP？");
    conversation.add_assistant("HTTP 是一种应用层协议。");
    conversation.push(Message::new(Role::User, "它基于什么传输？"));

    assert_eq!(
        conversation.messages(),
        [
            Message::system("你是一个助手。"),
            Message::user("什么是 HTTP？"),
            Message::assistant("HTTP 是一种应用层协议。"),
            Message::user("它基于什么传输？"),
        ]
    );
}
