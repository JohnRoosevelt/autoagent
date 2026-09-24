use std::collections::BTreeSet;

/// Operations that require an explicit host approval before they may be attempted.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Capability {
    FileWrite,
    FileDelete,
    CommandExecution,
    NetworkAccess,
}

#[derive(Clone, Debug, Default)]
pub struct PermissionPolicy {
    allowed: BTreeSet<Capability>,
}

impl PermissionPolicy {
    pub fn deny_all() -> Self {
        Self::default()
    }

    pub fn allow(mut self, capability: Capability) -> Self {
        self.allowed.insert(capability);
        self
    }

    pub fn check(&self, capability: &Capability) -> Result<(), PermissionError> {
        if self.allowed.contains(capability) {
            Ok(())
        } else {
            Err(PermissionError::Denied(capability.clone()))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PermissionError {
    #[error("permission denied: {0:?}")]
    Denied(Capability),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_every_sensitive_capability_by_default() {
        let policy = PermissionPolicy::deny_all();
        for capability in [
            Capability::FileWrite,
            Capability::FileDelete,
            Capability::CommandExecution,
            Capability::NetworkAccess,
        ] {
            assert!(matches!(
                policy.check(&capability),
                Err(PermissionError::Denied(_))
            ));
        }
    }

    #[test]
    fn grants_only_explicit_capabilities() {
        let policy = PermissionPolicy::deny_all().allow(Capability::FileWrite);
        assert!(policy.check(&Capability::FileWrite).is_ok());
        assert!(policy.check(&Capability::CommandExecution).is_err());
    }
}
