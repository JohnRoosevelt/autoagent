use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactRef {
    pub name: String,
    pub bytes: usize,
}
#[derive(Clone, Debug)]
pub struct ArtifactStore {
    root: PathBuf,
}
#[derive(Debug, thiserror::Error)]
pub enum ArtifactError {
    #[error("artifact root is invalid: {0}")]
    Root(#[source] std::io::Error),
    #[error("unsafe artifact name: {0}")]
    Unsafe(String),
    #[error("artifact already exists: {0}")]
    Exists(String),
    #[error("artifact operation failed: {0}")]
    Io(#[from] std::io::Error),
}
impl ArtifactStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, ArtifactError> {
        Ok(Self {
            root: root.as_ref().canonicalize().map_err(ArtifactError::Root)?,
        })
    }
    pub fn store(&self, name: &str, bytes: &[u8]) -> Result<ArtifactRef, ArtifactError> {
        if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
            return Err(ArtifactError::Unsafe(name.into()));
        }
        let path = self.root.join(name);
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::AlreadyExists {
                    ArtifactError::Exists(name.into())
                } else {
                    ArtifactError::Io(e)
                }
            })?;
        file.write_all(bytes)?;
        Ok(ArtifactRef {
            name: name.into(),
            bytes: bytes.len(),
        })
    }
    pub fn read(&self, reference: &ArtifactRef) -> Result<Vec<u8>, ArtifactError> {
        Ok(fs::read(self.root.join(&reference.name))?)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn externalizes_bytes_and_returns_a_small_reference() {
        let root = std::env::temp_dir().join(format!("autoagent-artifacts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let store = ArtifactStore::new(&root).unwrap();
        let reference = store.store("result.bin", b"large result").unwrap();
        assert_eq!(
            reference,
            ArtifactRef {
                name: "result.bin".into(),
                bytes: 12
            }
        );
        assert_eq!(store.read(&reference).unwrap(), b"large result");
        assert!(matches!(
            store.store("../bad", b"x"),
            Err(ArtifactError::Unsafe(_))
        ));
    }
}
