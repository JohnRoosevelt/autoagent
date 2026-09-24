use crate::tool::{ToolError, ToolRegistry};

/// Bounded dispatch for a single model tool-call batch.
///
/// Only tools that explicitly advertise `ParallelSafe` are scheduled concurrently. The
/// returned values remain in input order, so callers can safely append protocol tool
/// results in the order requested by the model.
pub async fn execute_safe_batch(
    registry: &ToolRegistry,
    calls: Vec<(String, String)>,
    max_parallel: usize,
) -> Result<Vec<Result<serde_json::Value, ToolError>>, ParallelError> {
    if max_parallel == 0 {
        return Err(ParallelError::InvalidLimit);
    }

    let mut results = Vec::with_capacity(calls.len());
    let mut safe_batch = Vec::new();
    for (name, arguments) in calls {
        if registry.is_parallel_safe(&name) {
            safe_batch.push((results.len(), name, arguments));
            results.push(None);
        } else {
            results.push(Some(registry.execute(&name, &arguments)));
        }
    }

    for chunk in safe_batch.chunks(max_parallel) {
        let mut tasks = Vec::with_capacity(chunk.len());
        for (index, name, arguments) in chunk {
            let tool = registry.tool_for_parallel(name)?;
            let arguments = serde_json::from_str(arguments)
                .map_err(|error| ParallelError::InvalidJson(error.to_string()))?;
            tasks.push((
                *index,
                tokio::task::spawn_blocking(move || tool.execute(arguments)),
            ));
        }
        for (index, task) in tasks {
            results[index] = Some(
                task.await
                    .map_err(|error| ParallelError::Task(error.to_string()))?,
            );
        }
    }

    Ok(results
        .into_iter()
        .map(|result| result.expect("each call has a result"))
        .collect())
}

#[derive(Debug, thiserror::Error)]
pub enum ParallelError {
    #[error("max_parallel must be greater than zero")]
    InvalidLimit,
    #[error("tool arguments are not valid JSON: {0}")]
    InvalidJson(String),
    #[error("parallel tool task failed: {0}")]
    Task(String),
    #[error(transparent)]
    Tool(#[from] ToolError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        llm::ToolDefinition,
        tool::{Tool, ToolExecution},
    };
    use serde_json::{Value, json};
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    struct TestTool {
        name: &'static str,
        execution: ToolExecution,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }
    impl Tool for TestTool {
        fn definition(&self) -> ToolDefinition {
            ToolDefinition::new(self.name, self.name, json!({}))
        }
        fn execution(&self) -> ToolExecution {
            self.execution
        }
        fn execute(&self, _arguments: Value) -> Result<Value, ToolError> {
            std::thread::sleep(Duration::from_millis(20));
            self.calls.lock().unwrap().push(self.name);
            Ok(json!(self.name))
        }
    }

    #[tokio::test]
    async fn only_explicitly_safe_tools_run_in_parallel_and_results_keep_input_order() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let mut registry = ToolRegistry::new();
        registry
            .register(TestTool {
                name: "safe_one",
                execution: ToolExecution::ParallelSafe,
                calls: calls.clone(),
            })
            .unwrap();
        registry
            .register(TestTool {
                name: "serial",
                execution: ToolExecution::Serial,
                calls: calls.clone(),
            })
            .unwrap();
        registry
            .register(TestTool {
                name: "safe_two",
                execution: ToolExecution::ParallelSafe,
                calls: calls.clone(),
            })
            .unwrap();
        let output = execute_safe_batch(
            &registry,
            vec![
                ("safe_one".into(), "{}".into()),
                ("serial".into(), "{}".into()),
                ("safe_two".into(), "{}".into()),
            ],
            2,
        )
        .await
        .unwrap();
        assert_eq!(
            output,
            vec![
                Ok(json!("safe_one")),
                Ok(json!("serial")),
                Ok(json!("safe_two"))
            ]
        );
        assert_eq!(
            calls
                .lock()
                .unwrap()
                .iter()
                .filter(|&&name| name == "serial")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn rejects_a_zero_parallel_limit() {
        assert!(matches!(
            execute_safe_batch(&ToolRegistry::new(), vec![], 0).await,
            Err(ParallelError::InvalidLimit)
        ));
    }
}
