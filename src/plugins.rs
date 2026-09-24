use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
/// Declarative metadata a host may register for an optional, already-linked capability.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
}
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin manifest name is empty")]
    EmptyName,
    #[error("plugin already registered: {0}")]
    Duplicate(String),
}
#[derive(Default)]
pub struct PluginRegistry {
    manifests: BTreeMap<String, PluginManifest>,
}
impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(&mut self, manifest: PluginManifest) -> Result<(), PluginError> {
        if manifest.name.trim().is_empty() {
            return Err(PluginError::EmptyName);
        }
        if self.manifests.contains_key(&manifest.name) {
            return Err(PluginError::Duplicate(manifest.name));
        }
        self.manifests.insert(manifest.name.clone(), manifest);
        Ok(())
    }
    pub fn manifests(&self) -> Vec<&PluginManifest> {
        self.manifests.values().collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn manifest(name: &str) -> PluginManifest {
        PluginManifest {
            name: name.into(),
            version: "1".into(),
            capabilities: vec!["tool:example".into()],
        }
    }
    #[test]
    fn registers_manifests_in_stable_order_without_loading_them() {
        let mut registry = PluginRegistry::new();
        registry.register(manifest("zeta")).unwrap();
        registry.register(manifest("alpha")).unwrap();
        assert_eq!(
            registry
                .manifests()
                .iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>(),
            ["alpha", "zeta"]
        );
        assert!(matches!(
            registry.register(manifest("alpha")),
            Err(PluginError::Duplicate(_))
        ));
    }
}
