use super::{chat_request_body, parse_chat_response};
use crate::{llm::LlmError, message::Conversation};

#[test]
fn second_round_request_contains_complete_conversation_context() {
    let mut conversation = Conversation::new();
    conversation.add_system("你是一个助手。");
    conversation.add_user("法国的首都是哪里？");
    conversation.add_assistant("法国的首都是巴黎。");
    conversation.add_user("它有多少人口？");

    let body = chat_request_body("test-model", conversation.messages());

    assert_eq!(body["model"], "test-model");
    assert_eq!(
        body["messages"],
        serde_json::json!([
            {"role": "system", "content": "你是一个助手。"},
            {"role": "user", "content": "法国的首都是哪里？"},
            {"role": "assistant", "content": "法国的首都是巴黎。"},
            {"role": "user", "content": "它有多少人口？"},
        ])
    );
}

#[test]
fn parses_openai_compatible_response() {
    let response = r#"{
        "choices": [
            { "message": { "content": "HTTP 是一种应用层协议。" } }
        ],
        "usage": {
            "prompt_tokens": 23,
            "completion_tokens": 12,
            "total_tokens": 35
        }
    }"#;

    let parsed = parse_chat_response(response).unwrap();
    assert_eq!(parsed.content, "HTTP 是一种应用层协议。");
    assert_eq!(parsed.usage.input_tokens, 23);
    assert_eq!(parsed.usage.output_tokens, 12);
}

#[test]
fn rejects_invalid_json_response() {
    let error = parse_chat_response("not json").unwrap_err();

    assert!(matches!(error, LlmError::Other(message) if message.contains("响应不是合法 JSON")));
}

#[test]
fn rejects_response_without_message_content() {
    let error = parse_chat_response(r#"{"choices": [{"message": {}}]}"#).unwrap_err();

    assert!(matches!(error, LlmError::Other(message) if message.contains("响应结构不符合预期")));
}

#[test]
fn rejects_response_without_usage() {
    let error = parse_chat_response(
        r#"{
        "choices": [{"message": {"content": "回答"}}]
    }"#,
    )
    .unwrap_err();

    assert!(
        matches!(error, LlmError::Other(message) if message.contains("响应缺少 usage.prompt_tokens"))
    );
}
