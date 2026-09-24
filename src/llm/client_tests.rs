use super::{SseParser, chat_request_body, parse_chat_response, stream_request_body};
use crate::{
    llm::{FinishReason, ToolCall, ToolDefinition},
    message::Conversation,
};

fn weather_tool() -> ToolDefinition {
    ToolDefinition::new(
        "weather",
        "查询天气",
        serde_json::json!({"type":"object","properties":{"city":{"type":"string"}}}),
    )
}

#[test]
fn requests_only_standard_openai_tools_when_definitions_are_present() {
    let body = chat_request_body("test-model", &[], &[weather_tool()]);
    assert_eq!(
        body["tools"],
        serde_json::json!([{"type":"function","function":{"name":"weather","description":"查询天气","parameters":{"type":"object","properties":{"city":{"type":"string"}}}}}])
    );
    assert!(body.get("tool_definitions").is_none());
    assert!(
        chat_request_body("test-model", &[], &[])
            .get("tools")
            .is_none()
    );
}

#[test]
fn stream_request_keeps_sse_usage_and_tools() {
    let mut conversation = Conversation::new();
    conversation.add_user("请查询");
    let body = stream_request_body("test-model", conversation.messages(), &[weather_tool()]);
    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);
    assert!(body["tools"].is_array());
}

#[test]
fn parses_text_response_finish_reason_and_usage() {
    let parsed = parse_chat_response(r#"{"choices":[{"finish_reason":"stop","message":{"content":"你好"}}],"usage":{"prompt_tokens":7,"completion_tokens":2}}"#).unwrap();
    assert_eq!(parsed.content, "你好");
    assert_eq!(parsed.finish_reason, FinishReason::Stop);
    assert!(parsed.tool_calls.is_empty());
    assert_eq!(parsed.usage.input_tokens, 7);
}

#[test]
fn parses_null_content_and_multiple_tool_calls() {
    let parsed = parse_chat_response(r#"{"choices":[{"finish_reason":"tool_calls","message":{"content":null,"tool_calls":[{"id":"call_1","function":{"name":"weather","arguments":"{\"city\":\"北京\"}"}},{"id":"call_2","function":{"name":"clock","arguments":"{}"}}]}}],"usage":{"prompt_tokens":7,"completion_tokens":2}}"#).unwrap();
    assert_eq!(parsed.content, "");
    assert_eq!(parsed.finish_reason, FinishReason::ToolCalls);
    assert_eq!(
        parsed.tool_calls,
        vec![
            ToolCall {
                id: "call_1".into(),
                name: "weather".into(),
                arguments: "{\"city\":\"北京\"}".into()
            },
            ToolCall {
                id: "call_2".into(),
                name: "clock".into(),
                arguments: "{}".into()
            }
        ]
    );
}

#[test]
fn aggregates_text_usage_and_fragmented_tool_calls_from_sse() {
    let chunks = [
        "data: {\"choices\":[{\"delta\":{\"content\":\"你\",\"tool_calls\":[{\"index\":0,\"id\":\"call_\",\"function\":{\"name\":\"wea\",\"arguments\":\"{\\\"city\\\":\"}}]}}]}\n\n",
        "data: {\"choices\":[{\"delta\":{\"content\":\"好\",\"tool_calls\":[{\"index\":0,\"id\":\"1\",\"function\":{\"name\":\"ther\",\"arguments\":\"\\\"北京\\\"}\"}},{\"index\":1,\"id\":\"call_2\",\"function\":{\"name\":\"clock\",\"arguments\":\"{}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\ndata: {\"choices\":[],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":2}}\n\ndata: [DONE]\n\n",
    ];
    let mut parser = SseParser::default();
    let mut displayed = String::new();
    for chunk in chunks {
        for delta in parser.push(chunk.as_bytes()).unwrap() {
            displayed.push_str(&delta);
        }
    }
    let (response, _) = parser.finish().unwrap();
    assert_eq!(displayed, "你好");
    assert_eq!(response.content, "你好");
    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
    assert_eq!(response.usage.output_tokens, 2);
    assert_eq!(
        response.tool_calls,
        vec![
            ToolCall {
                id: "call_1".into(),
                name: "weather".into(),
                arguments: "{\"city\":\"北京\"}".into()
            },
            ToolCall {
                id: "call_2".into(),
                name: "clock".into(),
                arguments: "{}".into()
            }
        ]
    );
}

#[test]
fn rejects_sse_stream_without_done_event() {
    let mut parser = SseParser::default();
    parser
        .push("data: {\"choices\":[{\"delta\":{\"content\":\"回答\"}}]}\n\n".as_bytes())
        .unwrap();
    assert!(parser.finish().is_err());
}
