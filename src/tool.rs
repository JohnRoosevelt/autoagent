use crate::{filesystem::Workspace, llm::ToolDefinition, shell::WorkspaceInspector};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

/// 一个编译进宿主程序的本地工具。
///
/// `ToolDefinition` 供模型选择工具；具体字段约束由工具自身验证，而非在本章实现
/// 完整 JSON Schema 引擎。
pub trait Tool: Send + Sync {
    fn definition(&self) -> ToolDefinition;
    /// Tools are serial unless they explicitly opt in. This is deliberately conservative:
    /// workspace, shell, and network tools must never become concurrent by accident.
    fn execution(&self) -> ToolExecution {
        ToolExecution::Serial
    }
    fn execute(&self, arguments: Value) -> Result<Value, ToolError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolExecution {
    Serial,
    ParallelSafe,
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
    tools: BTreeMap<String, Arc<dyn Tool>>,
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
        self.tools.insert(definition.name, Arc::new(tool));
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(Arc::as_ref)
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|tool| tool.definition()).collect()
    }

    pub fn is_parallel_safe(&self, name: &str) -> bool {
        self.get(name)
            .is_some_and(|tool| tool.execution() == ToolExecution::ParallelSafe)
    }

    pub(crate) fn tool_for_parallel(&self, name: &str) -> Result<Arc<dyn Tool>, ToolError> {
        self.tools
            .get(name)
            .cloned()
            .ok_or_else(|| ToolError::UnknownTool(name.into()))
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

pub struct ListFilesTool {
    workspace: Workspace,
}
pub struct ReadFileTool {
    workspace: Workspace,
}
pub struct CreateFileTool {
    workspace: Workspace,
}
pub struct OverwriteFileTool {
    workspace: Workspace,
}
pub struct RunInspectionTool {
    inspector: WorkspaceInspector,
}

impl ListFilesTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}
impl ReadFileTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}
impl CreateFileTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}
impl OverwriteFileTool {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
}
impl RunInspectionTool {
    pub fn new(workspace: Workspace) -> Self {
        Self {
            inspector: WorkspaceInspector::new(workspace),
        }
    }
}

fn string_argument(arguments: Value, field: &str) -> Result<String, ToolError> {
    arguments
        .as_object()
        .and_then(|object| object.get(field))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ToolError::InvalidArguments(format!("缺少非空字符串字段 {field}")))
}

impl Tool for ListFilesTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "list_files",
            "列出工作区目录的直接内容。",
            json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
        )
    }
    fn execute(&self, arguments: Value) -> Result<Value, ToolError> {
        Ok(
            json!({"entries": self.workspace.list(&string_argument(arguments, "path")?).map_err(|error| ToolError::Execution(error.to_string()))?}),
        )
    }
}
impl Tool for ReadFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "read_file",
            "读取工作区内一个 UTF-8 文本文件。",
            json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}),
        )
    }
    fn execute(&self, arguments: Value) -> Result<Value, ToolError> {
        Ok(
            json!({"content": self.workspace.read(&string_argument(arguments, "path")?).map_err(|error| ToolError::Execution(error.to_string()))?}),
        )
    }
}
impl Tool for CreateFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "create_file",
            "在工作区创建新文本文件；若文件已存在则拒绝。",
            json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"],"additionalProperties":false}),
        )
    }
    fn execute(&self, arguments: Value) -> Result<Value, ToolError> {
        let path = string_argument(arguments.clone(), "path")?;
        let content = string_argument(arguments, "content")?;
        self.workspace
            .create(&path, &content)
            .map_err(|error| ToolError::Execution(error.to_string()))?;
        Ok(json!({"created": path}))
    }
}
impl Tool for RunInspectionTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "run_inspection",
            "在工作区运行固定的只读检查：cargo_check、cargo_test、git_status、git_diff 或 git_log。",
            json!({"type":"object","properties":{"command":{"type":"string","enum":["cargo_check","cargo_test","git_status","git_diff","git_log"]}},"required":["command"],"additionalProperties":false}),
        )
    }
    fn execute(&self, arguments: Value) -> Result<Value, ToolError> {
        let command = string_argument(arguments, "command")?;
        let output = self
            .inspector
            .run_named(&command)
            .map_err(|error| ToolError::Execution(error.to_string()))?;
        Ok(json!({"success": output.success, "stdout": output.stdout, "stderr": output.stderr}))
    }
}

impl Tool for OverwriteFileTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition::new(
            "overwrite_file",
            "显式替换工作区内已有文本文件的全部内容。",
            json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"],"additionalProperties":false}),
        )
    }
    fn execute(&self, arguments: Value) -> Result<Value, ToolError> {
        let path = string_argument(arguments.clone(), "path")?;
        let content = string_argument(arguments, "content")?;
        self.workspace
            .overwrite(&path, &content)
            .map_err(|error| ToolError::Execution(error.to_string()))?;
        Ok(json!({"overwritten": path}))
    }
}

#[cfg(test)]
#[path = "tool_tests.rs"]
mod tests;
