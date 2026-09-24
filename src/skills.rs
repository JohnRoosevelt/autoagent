use std::{
    fs,
    path::{Path, PathBuf},
};

/// On-demand Markdown skill definitions confined to one canonical directory.
#[derive(Clone, Debug)]
pub struct SkillDirectory {
    root: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("skill root is invalid: {0}")]
    InvalidRoot(#[source] std::io::Error),
    #[error("unsafe skill name: {0}")]
    UnsafeName(String),
    #[error("skill is not available: {0}")]
    NotFound(String),
    #[error("skill file must be UTF-8 Markdown: {0}")]
    InvalidDefinition(String),
    #[error("skill operation failed: {0}")]
    Io(#[from] std::io::Error),
}

impl SkillDirectory {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, SkillError> {
        Ok(Self {
            root: root
                .as_ref()
                .canonicalize()
                .map_err(SkillError::InvalidRoot)?,
        })
    }
    pub fn list(&self) -> Result<Vec<String>, SkillError> {
        let mut names = fs::read_dir(&self.root)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                (path.extension().and_then(|x| x.to_str()) == Some("md") && path.is_file())
                    .then(|| path.file_stem()?.to_str().map(str::to_owned))
                    .flatten()
            })
            .collect::<Vec<_>>();
        names.sort();
        Ok(names)
    }
    pub fn load(&self, name: &str) -> Result<String, SkillError> {
        if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
            return Err(SkillError::UnsafeName(name.into()));
        }
        let path = self.root.join(format!("{name}.md"));
        let canonical = path
            .canonicalize()
            .map_err(|_| SkillError::NotFound(name.into()))?;
        if !canonical.starts_with(&self.root) {
            return Err(SkillError::UnsafeName(name.into()));
        }
        let text = fs::read_to_string(canonical)?;
        if !text.starts_with('#') {
            return Err(SkillError::InvalidDefinition(name.into()));
        }
        Ok(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "autoagent-skills-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }
    #[test]
    fn lists_and_loads_markdown_on_demand() {
        let path = root();
        fs::write(path.join("review.md"), "# Review\nCheck tests.").unwrap();
        fs::write(path.join("ignored.txt"), "x").unwrap();
        let skills = SkillDirectory::new(&path).unwrap();
        assert_eq!(skills.list().unwrap(), ["review"]);
        assert_eq!(skills.load("review").unwrap(), "# Review\nCheck tests.");
    }
    #[test]
    fn rejects_path_escape_and_non_definition() {
        let path = root();
        fs::write(path.join("plain.md"), "not a heading").unwrap();
        let skills = SkillDirectory::new(&path).unwrap();
        assert!(matches!(
            skills.load("../secret"),
            Err(SkillError::UnsafeName(_))
        ));
        assert!(matches!(
            skills.load("plain"),
            Err(SkillError::InvalidDefinition(_))
        ));
    }
}
