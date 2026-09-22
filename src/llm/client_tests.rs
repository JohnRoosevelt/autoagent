use super::{
    SseParser, chat_request_body, parse_chat_response, send_stream_event, stream_request_body,
};
use crate::{
    llm::{ChatResponse, LlmError, StreamEvent, Usage},
    message::Conversation,
};
use tokio::sync::mpsc;

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
fn stream_request_enables_sse_and_usage_reporting() {
    let mut conversation = Conversation::new();
    conversation.add_user("请流式回答");

    let body = stream_request_body("test-model", conversation.messages());

    assert_eq!(body["stream"], true);
    assert_eq!(body["stream_options"]["include_usage"], true);
}

#[test]
fn parses_sse_deltas_across_http_chunk_boundaries() {
    let chunks = [
        "data: {\"choices\":[{\"delta\":{\"content\":\"你\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"好\"}}]}".as_bytes(),
        "\n\ndata: {\"choices\":[],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":2}}\n".as_bytes(),
        "\ndata: [DONE]\n\n".as_bytes(),
    ];
    let mut parser = SseParser::default();
    let mut displayed = String::new();
    for chunk in chunks {
        for delta in parser.push(chunk).unwrap() {
            displayed.push_str(&delta);
        }
    }
    let (response, final_deltas) = parser.finish().unwrap();
    for delta in final_deltas {
        displayed.push_str(&delta);
    }

    assert_eq!(displayed, "你好");
    assert_eq!(response.content, "你好");
    assert_eq!(response.usage.input_tokens, 7);
    assert_eq!(response.usage.output_tokens, 2);
}

#[test]
fn rejects_sse_stream_without_done_event() {
    let mut parser = SseParser::default();
    parser
        .push("data: {\"choices\":[{\"delta\":{\"content\":\"回答\"}}]}\n\n".as_bytes())
        .unwrap();

    let error = parser.finish().unwrap_err();
    assert!(matches!(error, LlmError::Other(message) if message.contains("[DONE]")));
}

#[tokio::test]
async fn delivers_stream_events_in_order() {
    let (tx, mut rx) = mpsc::channel(3);
    let response = ChatResponse {
        content: "你好".into(),
        usage: Usage {
            input_tokens: 7,
            output_tokens: 2,
        },
    };

    send_stream_event(&tx, StreamEvent::Start).await.unwrap();
    send_stream_event(&tx, StreamEvent::TextDelta("你".into()))
        .await
        .unwrap();
    send_stream_event(&tx, StreamEvent::Done(response.clone()))
        .await
        .unwrap();
    drop(tx);

    assert_eq!(rx.recv().await, Some(StreamEvent::Start));
    assert_eq!(rx.recv().await, Some(StreamEvent::TextDelta("你".into())));
    assert_eq!(rx.recv().await, Some(StreamEvent::Done(response)));
    assert_eq!(rx.recv().await, None);
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
