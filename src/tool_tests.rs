use super::{GetWeatherTool, Tool, ToolError, ToolRegistry};
use crate::llm::ToolDefinition;
use serde_json::{Value, json};

struct EchoTool;

impl Tool for EchoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new("echo", "回显", json!({"type":"object"}))
    }

    fn execute(&self, arguments: Value) -> Result<Value, ToolError> {
        Ok(arguments)
    }
}

#[test]
fn registers_finds_and_exports_definitions_in_name_order() {
    let mut registry = ToolRegistry::new();
    registry.register(GetWeatherTool).unwrap();
    registry.register(EchoTool).unwrap();

    assert!(registry.get("get_weather").is_some());
    assert!(registry.get("missing").is_none());
    assert_eq!(
        registry
            .definitions()
            .into_iter()
            .map(|definition| definition.name)
            .collect::<Vec<_>>(),
        ["echo", "get_weather"]
    );
}

#[test]
fn rejects_duplicate_tool_names() {
    let mut registry = ToolRegistry::new();
    registry.register(GetWeatherTool).unwrap();
    assert_eq!(
        registry.register(GetWeatherTool),
        Err(ToolError::DuplicateName("get_weather".into()))
    );
}

#[test]
fn executes_the_fixed_weather_tool() {
    let mut registry = ToolRegistry::new();
    registry.register(GetWeatherTool).unwrap();

    assert_eq!(
        registry
            .execute("get_weather", r#"{"city":"北京"}"#)
            .unwrap(),
        json!({"city":"北京","condition":"晴","temperature_c":20,"source":"fixed-demo"})
    );
}

#[test]
fn reports_invalid_json_missing_or_wrong_typed_arguments() {
    let mut registry = ToolRegistry::new();
    registry.register(GetWeatherTool).unwrap();

    assert!(matches!(
        registry.execute("get_weather", "{"),
        Err(ToolError::InvalidJson(_))
    ));
    assert!(matches!(
        registry.execute("get_weather", "{}"),
        Err(ToolError::InvalidArguments(message)) if message == "缺少字符串字段 city"
    ));
    assert!(matches!(
        registry.execute("get_weather", r#"{"city":42}"#),
        Err(ToolError::InvalidArguments(message)) if message == "缺少字符串字段 city"
    ));
}

#[test]
fn reports_unknown_tools_before_parsing_arguments() {
    let registry = ToolRegistry::new();
    assert_eq!(
        registry.execute("missing", "{"),
        Err(ToolError::UnknownTool("missing".into()))
    );
}
