use std::{
    fs,
    path::{Path, PathBuf},
};

/// File-backed Markdown memory. Callers explicitly select scopes to inject.
#[derive(Clone, Debug)]
pub struct MemoryStore {
    root: PathBuf,
}
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("memory root is invalid: {0}")]
    Root(#[source] std::io::Error),
    #[error("unsafe memory scope: {0}")]
    Unsafe(String),
    #[error("memory is unavailable: {0}")]
    Missing(String),
    #[error("memory operation failed: {0}")]
    Io(#[from] std::io::Error),
}
impl MemoryStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, MemoryError> {
        Ok(Self {
            root: root.as_ref().canonicalize().map_err(MemoryError::Root)?,
        })
    }
    pub fn inject(&self, scopes: &[&str]) -> Result<Vec<String>, MemoryError> {
        scopes.iter().map(|scope| self.read(scope)).collect()
    }
    fn read(&self, scope: &str) -> Result<String, MemoryError> {
        if scope.is_empty() || scope.contains(['\\', '/']) || scope.contains("..") {
            return Err(MemoryError::Unsafe(scope.into()));
        }
        let path = self
            .root
            .join(format!("{scope}.md"))
            .canonicalize()
            .map_err(|_| MemoryError::Missing(scope.into()))?;
        if !path.starts_with(&self.root) {
            return Err(MemoryError::Unsafe(scope.into()));
        }
        Ok(fs::read_to_string(path)?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn injects_selected_hierarchical_files_in_order() {
        let root = std::env::temp_dir().join(format!("autoagent-memory-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("global.md"), "global").unwrap();
        fs::write(root.join("file.md"), "file").unwrap();
        let store = MemoryStore::new(&root).unwrap();
        assert_eq!(
            store.inject(&["global", "file"]).unwrap(),
            ["global", "file"]
        );
        assert!(matches!(
            store.inject(&["../secret"]),
            Err(MemoryError::Unsafe(_))
        ));
    }
}
