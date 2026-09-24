use std::{
    fs,
    path::{Path, PathBuf},
};

/// File access rooted at one canonical workspace directory.
#[derive(Clone, Debug)]
pub struct Workspace {
    root: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace root is invalid: {0}")]
    InvalidRoot(#[source] std::io::Error),
    #[error("path must be a relative workspace path: {0}")]
    UnsafePath(String),
    #[error("file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("refusing to overwrite an existing file: {0}")]
    AlreadyExists(String),
    #[error("refusing to read a directory: {0}")]
    IsDirectory(String),
}

impl Workspace {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, WorkspaceError> {
        Ok(Self {
            root: root
                .as_ref()
                .canonicalize()
                .map_err(WorkspaceError::InvalidRoot)?,
        })
    }

    pub fn list(&self, path: &str) -> Result<Vec<String>, WorkspaceError> {
        let directory = self.resolve_existing(path)?;
        if !directory.is_dir() {
            return Err(WorkspaceError::IsDirectory(path.into()));
        }
        let mut entries = fs::read_dir(directory)?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        Ok(entries)
    }

    pub fn read(&self, path: &str) -> Result<String, WorkspaceError> {
        let file = self.resolve_existing(path)?;
        if file.is_dir() {
            return Err(WorkspaceError::IsDirectory(path.into()));
        }
        Ok(fs::read_to_string(file)?)
    }

    /// Creates only a new file. Existing files are never changed through this method.
    pub fn create(&self, path: &str, content: &str) -> Result<(), WorkspaceError> {
        let file = self.resolve_for_write(path)?;
        if file.exists() {
            return Err(WorkspaceError::AlreadyExists(path.into()));
        }
        fs::write(file, content)?;
        Ok(())
    }

    /// Replaces an existing file only when the caller explicitly opts in.
    pub fn overwrite(&self, path: &str, content: &str) -> Result<(), WorkspaceError> {
        let file = self.resolve_existing(path)?;
        if file.is_dir() {
            return Err(WorkspaceError::IsDirectory(path.into()));
        }
        fs::write(file, content)?;
        Ok(())
    }

    fn resolve_existing(&self, path: &str) -> Result<PathBuf, WorkspaceError> {
        let candidate = self.lexical_path(path)?;
        let canonical = candidate.canonicalize()?;
        self.ensure_contained(canonical)
    }

    fn resolve_for_write(&self, path: &str) -> Result<PathBuf, WorkspaceError> {
        let candidate = self.lexical_path(path)?;
        let parent = candidate
            .parent()
            .ok_or_else(|| WorkspaceError::UnsafePath(path.into()))?;
        self.ensure_contained(parent.canonicalize()?)?;
        Ok(candidate)
    }

    fn lexical_path(&self, path: &str) -> Result<PathBuf, WorkspaceError> {
        let relative = Path::new(path);
        if relative.as_os_str().is_empty()
            || relative.is_absolute()
            || relative
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(WorkspaceError::UnsafePath(path.into()));
        }
        Ok(self.root.join(relative))
    }

    fn ensure_contained(&self, path: PathBuf) -> Result<PathBuf, WorkspaceError> {
        if path.starts_with(&self.root) {
            Ok(path)
        } else {
            Err(WorkspaceError::UnsafePath(path.display().to_string()))
        }
    }
}

#[cfg(test)]
#[path = "filesystem_tests.rs"]
mod tests;
