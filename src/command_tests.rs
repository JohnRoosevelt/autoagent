use super::{Command, CommandError, parse_args};

#[test]
fn parses_slash_commands_without_starting_an_agent() {
    assert_eq!(parse_args(vec!["/help".into()]), Ok(Command::Help));
    assert_eq!(parse_args(vec!["/status".into()]), Ok(Command::Status));
    assert_eq!(parse_args(vec!["/exit".into()]), Ok(Command::Exit));
}

#[test]
fn joins_positional_arguments_as_one_prompt() {
    assert_eq!(
        parse_args(vec!["explain".into(), "this".into()]),
        Ok(Command::Prompt("explain this".into()))
    );
}

#[test]
fn rejects_unknown_slash_commands() {
    assert_eq!(
        parse_args(vec!["/delete-everything".into()]),
        Err(CommandError::UnknownSlashCommand(
            "/delete-everything".into()
        ))
    );
}
