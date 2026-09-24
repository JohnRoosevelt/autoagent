use std::path::{Path, PathBuf};

/// Declarative boundary for a future OS/container sandbox. It is not an executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SandboxProfile {
    pub workspace_root: PathBuf,
    pub network_enabled: bool,
    pub read_only: bool,
}

impl SandboxProfile {
    pub fn restricted(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
            network_enabled: false,
            read_only: true,
        }
    }

    pub fn validates_root(&self) -> bool {
        self.workspace_root.is_absolute() && Path::new(&self.workspace_root).is_dir()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("sandbox execution is intentionally unavailable")]
    ExecutionUnavailable,
}

/// Refuses all execution until a separately reviewed OS/container backend exists.
pub fn execute_untrusted(_profile: &SandboxProfile, _program: &str) -> Result<(), SandboxError> {
    Err(SandboxError::ExecutionUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restricted_profile_disables_network_and_writes() {
        let profile = SandboxProfile::restricted(std::env::temp_dir());
        assert!(profile.validates_root());
        assert!(!profile.network_enabled);
        assert!(profile.read_only);
    }

    #[test]
    fn refuses_to_execute_untrusted_programs() {
        let profile = SandboxProfile::restricted(std::env::temp_dir());
        assert!(matches!(
            execute_untrusted(&profile, "untrusted"),
            Err(SandboxError::ExecutionUnavailable)
        ));
    }
}
