use crate::{
    agent::{Agent, AgentError, AgentEvent, RetryPolicy, RunReport, StreamChatModel},
    configuration::AppConfig,
    filesystem::{Workspace, WorkspaceError},
    message::Conversation,
    permission::{Capability, PermissionError, PermissionPolicy},
    tool::{
        CreateFileTool, GetWeatherTool, ListFilesTool, OverwriteFileTool, ReadFileTool,
        RunInspectionTool, ToolError, ToolRegistry,
    },
};

use tokio::sync::mpsc;

/// Application-level composition of a model, policy, workspace tools, and Agent loop.
///
/// The framework deliberately reuses the existing `Agent` event channel rather than
/// introducing a second event bus. Sensitive capabilities remain opt-in.
#[allow(dead_code)]
pub struct App<M> {
    agent: Agent<M>,
    config: AppConfig,
    policy: PermissionPolicy,
    workspace: Workspace,
}

#[allow(dead_code)]
impl<M: StreamChatModel> App<M> {
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn policy(&self) -> &PermissionPolicy {
        &self.policy
    }

    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    pub fn agent(&self) -> &Agent<M> {
        &self.agent
    }

    pub fn enqueue_user(&mut self, content: impl Into<String>) {
        self.agent.enqueue_user(content);
    }

    /// Runs through the existing lifecycle event channel after approving model network access.
    pub async fn run(
        &mut self,
        events: mpsc::Sender<AgentEvent>,
    ) -> Result<RunReport, FrameworkError> {
        self.policy.check(&Capability::NetworkAccess)?;
        Ok(self.agent.run(events).await?)
    }
}

/// Builder for the stable host composition used by the CLI and embedding applications.
pub struct AgentBuilder<M> {
    model: M,
    config: AppConfig,
    policy: PermissionPolicy,
    workspace: Workspace,
    system_prompt: Option<String>,
    retry_policy: RetryPolicy,
    history_message_budget: usize,
}

impl<M: StreamChatModel> AgentBuilder<M> {
    pub fn new(model: M, config: AppConfig, workspace: Workspace) -> Self {
        Self {
            model,
            config,
            policy: PermissionPolicy::deny_all(),
            workspace,
            system_prompt: None,
            retry_policy: RetryPolicy::default(),
            history_message_budget: usize::MAX,
        }
    }

    pub fn with_policy(mut self, policy: PermissionPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    pub fn with_history_message_budget(mut self, max_messages: usize) -> Self {
        self.history_message_budget = max_messages;
        self
    }

    /// Registers deterministic and read-only tools by default. File mutation and workspace
    /// inspection are added only when their corresponding capabilities are approved.
    pub fn build(self) -> Result<App<M>, FrameworkError> {
        let mut conversation = Conversation::new();
        if let Some(prompt) = self.system_prompt {
            conversation.add_system(prompt);
        }

        let mut tools = ToolRegistry::new();
        tools.register(GetWeatherTool)?;
        tools.register(ListFilesTool::new(self.workspace.clone()))?;
        tools.register(ReadFileTool::new(self.workspace.clone()))?;
        if self.policy.check(&Capability::FileWrite).is_ok() {
            tools.register(CreateFileTool::new(self.workspace.clone()))?;
            tools.register(OverwriteFileTool::new(self.workspace.clone()))?;
        }
        if self.policy.check(&Capability::CommandExecution).is_ok() {
            tools.register(RunInspectionTool::new(self.workspace.clone()))?;
        }

        let agent = Agent::new(self.model, conversation, self.config.max_steps)?
            .with_tool_registry(tools)
            .with_retry_policy(self.retry_policy)
            .with_history_message_budget(self.history_message_budget);
        Ok(App {
            agent,
            config: self.config,
            policy: self.policy,
            workspace: self.workspace,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FrameworkError {
    #[error(transparent)]
    Agent(#[from] AgentError),
    #[error(transparent)]
    Permission(#[from] PermissionError),
    #[error(transparent)]
    Tool(#[from] ToolError),
    #[error(transparent)]
    Workspace(#[from] WorkspaceError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        llm::{ChatResponse, LlmError, StreamEvent, ToolDefinition},
        message::Message,
    };
    use std::{
        future::Future,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[derive(Clone)]
    struct FakeModel;

    impl StreamChatModel for FakeModel {
        fn chat_stream(
            &self,
            _messages: &[Message],
            _tools: &[ToolDefinition],
            _events: mpsc::Sender<StreamEvent>,
        ) -> impl Future<Output = Result<ChatResponse, LlmError>> + Send {
            async {
                Ok(ChatResponse {
                    content: String::new(),
                    tool_calls: Vec::new(),
                    finish_reason: crate::llm::FinishReason::Stop,
                    usage: crate::llm::Usage::default(),
                })
            }
        }
    }

    fn workspace() -> Workspace {
        let root = std::env::temp_dir().join(format!(
            "autoagent-framework-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        Workspace::new(root).unwrap()
    }

    fn tool_names(app: &App<FakeModel>) -> Vec<String> {
        app.agent()
            .tool_definitions()
            .into_iter()
            .map(|tool| tool.name)
            .collect()
    }

    #[test]
    fn defaults_to_read_only_workspace_tools() {
        let app = AgentBuilder::new(FakeModel, AppConfig::default(), workspace())
            .with_system_prompt("be concise")
            .build()
            .unwrap();
        assert_eq!(
            app.agent().conversation().messages(),
            [Message::system("be concise")]
        );
        assert_eq!(tool_names(&app), ["get_weather", "list_files", "read_file"]);
    }

    #[test]
    fn adds_sensitive_tools_only_for_approved_capabilities() {
        let policy = PermissionPolicy::deny_all()
            .allow(Capability::FileWrite)
            .allow(Capability::CommandExecution)
            .allow(Capability::NetworkAccess);
        let app = AgentBuilder::new(FakeModel, AppConfig::default(), workspace())
            .with_policy(policy)
            .build()
            .unwrap();
        assert_eq!(
            tool_names(&app),
            [
                "create_file",
                "get_weather",
                "list_files",
                "overwrite_file",
                "read_file",
                "run_inspection",
            ]
        );
    }

    #[tokio::test]
    async fn refuses_to_run_without_network_approval() {
        let mut app = AgentBuilder::new(FakeModel, AppConfig::default(), workspace())
            .build()
            .unwrap();
        let (events, _receiver) = mpsc::channel(1);
        assert!(matches!(
            app.run(events).await,
            Err(FrameworkError::Permission(PermissionError::Denied(
                Capability::NetworkAccess
            )))
        ));
    }
}
