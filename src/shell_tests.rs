use super::{InspectionCommand, ShellError, WorkspaceInspector};
use crate::filesystem::Workspace;

#[test]
fn only_allowlisted_inspection_commands_parse() {
    assert_eq!(
        InspectionCommand::parse("git_status"),
        Some(InspectionCommand::GitStatus)
    );
    assert_eq!(InspectionCommand::parse("rm -rf ."), None);
}

#[test]
fn rejects_arbitrary_command_names() {
    let workspace = Workspace::new(std::env::current_dir().unwrap()).unwrap();
    assert!(matches!(
        WorkspaceInspector::new(workspace).run_named("git reset --hard"),
        Err(ShellError::Unsupported(_))
    ));
}
