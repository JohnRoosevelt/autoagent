use crate::{
    llm::ToolDefinition,
    tool::{Tool, ToolError},
};
use serde_json::Value;
/// Local callbacks around exactly one tool invocation.
pub trait ToolHook: Send + Sync {
    fn before(&self, name: &str, arguments: &Value) -> Result<(), ToolError>;
    fn after(&self, name: &str, result: &Result<Value, ToolError>);
}
pub struct HookedTool<T, H> {
    tool: T,
    hook: H,
}
impl<T, H> HookedTool<T, H> {
    pub fn new(tool: T, hook: H) -> Self {
        Self { tool, hook }
    }
}
impl<T: Tool, H: ToolHook> Tool for HookedTool<T, H> {
    fn definition(&self) -> ToolDefinition {
        self.tool.definition()
    }
    fn execute(&self, arguments: Value) -> Result<Value, ToolError> {
        let name = self.tool.definition().name;
        self.hook.before(&name, &arguments)?;
        let result = self.tool.execute(arguments);
        self.hook.after(&name, &result);
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    struct TestTool;
    impl Tool for TestTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition::new("test", "", json!({}))
        }
        fn execute(&self, _: Value) -> Result<Value, ToolError> {
            Ok(json!("ok"))
        }
    }
    struct Recorder(Arc<Mutex<Vec<String>>>);
    impl ToolHook for Recorder {
        fn before(&self, name: &str, _: &Value) -> Result<(), ToolError> {
            self.0.lock().unwrap().push(format!("before:{name}"));
            Ok(())
        }
        fn after(&self, name: &str, _: &Result<Value, ToolError>) {
            self.0.lock().unwrap().push(format!("after:{name}"));
        }
    }
    #[test]
    fn calls_pre_and_post_callbacks() {
        let log = Arc::new(Mutex::new(vec![]));
        let wrapped = HookedTool::new(TestTool, Recorder(log.clone()));
        assert_eq!(wrapped.execute(json!({})).unwrap(), json!("ok"));
        assert_eq!(*log.lock().unwrap(), ["before:test", "after:test"]);
    }
}
