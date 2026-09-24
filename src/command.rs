/// Minimal dependency-free command parser for the terminal entry point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    Prompt(String),
    Help,
    Status,
    Exit,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CommandError {
    #[error("unknown slash command: {0}")]
    UnknownSlashCommand(String),
}

pub fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Command, CommandError> {
    let words: Vec<_> = args.into_iter().collect();
    match words.as_slice() {
        [] => Ok(Command::Prompt(
            "请查询北京现在的天气。请调用 get_weather，不要猜测结果。".into(),
        )),
        [command] if command == "/help" || command == "--help" || command == "-h" => {
            Ok(Command::Help)
        }
        [command] if command == "/status" => Ok(Command::Status),
        [command] if command == "/exit" || command == "/quit" => Ok(Command::Exit),
        [command, ..] if command.starts_with('/') => {
            Err(CommandError::UnknownSlashCommand(command.clone()))
        }
        _ => Ok(Command::Prompt(words.join(" "))),
    }
}

pub fn help_text() -> &'static str {
    "Commands: /help, /status, /exit. Any non-slash arguments are sent as one prompt."
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
