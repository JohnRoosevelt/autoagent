use super::{Workspace, WorkspaceError};
use std::{fs, path::PathBuf};

fn root(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("autoagent-workspace-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn lists_reads_and_creates_only_inside_the_workspace() {
    let root = root("basic");
    fs::write(root.join("a.txt"), "hello").unwrap();
    let workspace = Workspace::new(&root).unwrap();
    assert_eq!(workspace.list(".").unwrap(), vec!["a.txt"]);
    assert_eq!(workspace.read("a.txt").unwrap(), "hello");
    workspace.create("new.txt", "new").unwrap();
    assert_eq!(workspace.read("new.txt").unwrap(), "new");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rejects_absolute_and_parent_traversal_paths() {
    let root = root("traversal");
    let workspace = Workspace::new(&root).unwrap();
    for path in ["../outside", "/tmp/outside"] {
        assert!(matches!(
            workspace.read(path),
            Err(WorkspaceError::UnsafePath(_))
        ));
        assert!(matches!(
            workspace.create(path, "x"),
            Err(WorkspaceError::UnsafePath(_))
        ));
    }
    let _ = fs::remove_dir_all(root);
}

#[test]
fn creation_does_not_overwrite_without_explicit_operation() {
    let root = root("overwrite");
    fs::write(root.join("file.txt"), "old").unwrap();
    let workspace = Workspace::new(&root).unwrap();
    assert!(matches!(
        workspace.create("file.txt", "new"),
        Err(WorkspaceError::AlreadyExists(_))
    ));
    workspace.overwrite("file.txt", "new").unwrap();
    assert_eq!(workspace.read("file.txt").unwrap(), "new");
    let _ = fs::remove_dir_all(root);
}
