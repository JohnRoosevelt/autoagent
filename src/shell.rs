use crate::filesystem::Workspace;
use std::process::Command;

/// A fixed, inspection-only command set. No shell is invoked.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectionCommand {
    CargoCheck,
    CargoTest,
    GitStatus,
    GitDiff,
    GitLog,
}

impl InspectionCommand {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "cargo_check" => Some(Self::CargoCheck),
            "cargo_test" => Some(Self::CargoTest),
            "git_status" => Some(Self::GitStatus),
            "git_diff" => Some(Self::GitDiff),
            "git_log" => Some(Self::GitLog),
            _ => None,
        }
    }

    fn program_and_args(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::CargoCheck => ("cargo", &["check", "--offline"]),
            Self::CargoTest => ("cargo", &["test", "--offline"]),
            Self::GitStatus => ("git", &["--no-optional-locks", "status", "--short"]),
            Self::GitDiff => ("git", &["--no-pager", "diff", "--no-ext-diff"]),
            Self::GitLog => ("git", &["--no-pager", "log", "--oneline", "-n", "20"]),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    #[error("unsupported inspection command: {0}")]
    Unsupported(String),
    #[error("failed to start inspection command: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Debug)]
pub struct WorkspaceInspector {
    workspace: Workspace,
}
impl WorkspaceInspector {
    pub fn new(workspace: Workspace) -> Self {
        Self { workspace }
    }
    pub fn run(&self, command: InspectionCommand) -> Result<CommandOutput, ShellError> {
        let (program, args) = command.program_and_args();
        let output = Command::new(program)
            .args(args)
            .current_dir(self.workspace.root())
            .output()?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
    pub fn run_named(&self, command: &str) -> Result<CommandOutput, ShellError> {
        self.run(
            InspectionCommand::parse(command)
                .ok_or_else(|| ShellError::Unsupported(command.into()))?,
        )
    }
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod tests;
