use super::parse_chat_response;
use crate::llm::LlmError;

#[test]
fn parses_openai_compatible_response() {
    let response = r#"{
        "choices": [
            { "message": { "content": "HTTP 是一种应用层协议。" } }
        ]
    }"#;

    assert_eq!(
        parse_chat_response(response).unwrap(),
        "HTTP 是一种应用层协议。"
    );
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
