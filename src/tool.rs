use crate::llm::ToolDefinition;
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// 一个编译进宿主程序的本地工具。
///
/// `ToolDefinition` 供模型选择工具；具体字段约束由工具自身验证，而非在本章实现
/// 完整 JSON Schema 引擎。
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    fn execute(&self, arguments: Value) -> Result<Value, ToolError>;
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ToolError {
    #[error("工具名称已注册: {0}")]
    DuplicateName(String),
    #[error("未知工具: {0}")]
    UnknownTool(String),
    #[error("工具参数不是合法 JSON: {0}")]
    InvalidJson(String),
    #[error("工具参数无效: {0}")]
    InvalidArguments(String),
    /// 保留给具体工具报告其运行时失败；演示工具是确定性的，当前不会构造该分支。
    #[allow(dead_code)]
    #[error("工具执行失败: {0}")]
    Execution(String),
}

impl ToolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DuplicateName(_) => "duplicate_tool_name",
            Self::UnknownTool(_) => "unknown_tool",
            Self::InvalidJson(_) => "invalid_json",
            Self::InvalidArguments(_) => "invalid_arguments",
            Self::Execution(_) => "execution_failed",
        }
    }
}

/// 本地工具的运行时注册表。
///
/// 这是进程启动时组装的内存注册表，不涉及动态库、目录发现或热加载。`BTreeMap`
/// 使导出给模型的工具定义顺序稳定，便于测试和重现请求。
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T: Tool + 'static>(&mut self, tool: T) -> Result<(), ToolError> {
        let definition = tool.definition();
        if self.tools.contains_key(&definition.name) {
            return Err(ToolError::DuplicateName(definition.name));
        }
        self.tools.insert(definition.name, Box::new(tool));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(Box::as_ref)
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| tool.definition()).collect()
    }

    /// 查找、解析原始 arguments 并执行工具。工具实现负责字段级校验。
    pub fn execute(&self, name: &str, raw_arguments: &str) -> Result<Value, ToolError> {
        let tool = self
            .get(name)
            .ok_or_else(|| ToolError::UnknownTool(name.into()))?;
        let arguments = serde_json::from_str(raw_arguments)
            .map_err(|error| ToolError::InvalidJson(error.to_string()))?;
        tool.execute(arguments)
    }
}

/// 完全离线、确定性的演示工具；不访问网络或系统资源。
pub struct GetWeatherTool;

impl Tool for GetWeatherTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "get_weather",
            "查询指定城市的当前天气。",
            json!({
                "type": "object",
                "properties": { "city": { "type": "string", "description": "城市名称" } },
                "required": ["city"],
                "additionalProperties": false
            }),
        )
    }

    fn execute(&self, arguments: Value) -> Result<Value, ToolError> {
        let object = arguments
            .as_object()
            .ok_or_else(|| ToolError::InvalidArguments("arguments 必须是对象".into()))?;
        let city = object
            .get("city")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidArguments("缺少字符串字段 city".into()))?;
        if city.trim().is_empty() {
            return Err(ToolError::InvalidArguments("字段 city 不能为空".into()));
        }
        if object.len() != 1 {
            return Err(ToolError::InvalidArguments("不支持未声明的字段".into()));
        }
        Ok(json!({
            "city": city,
            "condition": "晴",
            "temperature_c": 20,
            "source": "fixed-demo"
        }))
    }
}

#[cfg(test)]
#[path = "tool_tests.rs"]
mod tests;
