use std::fmt;

/// Parsed UCI command issued by a GUI, harness, or CLI user.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum UciCommand {
    /// Request engine identification and supported options (`uci`).
    Uci,

    /// Synchronization ping expecting `readyok` (`isready`).
    IsReady,

    /// Reset internal engine state for a new game (`ucinewgame`).
    UciNewGame,

    /// Configure an engine option (`setoption name <id> [value <val>]`).
    SetOption {
        /// Option name.
        name: String,

        /// Optional option value.
        value: Option<String>,
    },

    /// Terminate the UCI loop (`quit`).
    Quit,
}

/// Errors produced while parsing a UCI input line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseCommandError {
    /// Line contained only whitespace.
    EmptyLine,

    /// Command keyword is not recognized.
    UnknownCommand(String),

    /// Command arguments could not be parsed cleanly.
    InvalidSyntax(&'static str),
}

impl fmt::Display for ParseCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLine => write!(f, "empty command line"),
            Self::UnknownCommand(cmd) => write!(f, "unknown UCI command: {cmd}"),
            Self::InvalidSyntax(msg) => write!(f, "invalid UCI syntax: {msg}"),
        }
    }
}

impl std::error::Error for ParseCommandError {}

/// Parses a single UCI input line into a [`UciCommand`].
///
/// # Errors
///
/// Returns [`ParseCommandError::EmptyLine`] on blank input, [`ParseCommandError::UnknownCommand`]
/// if the leading keyword is unsupported, or [`ParseCommandError::InvalidSyntax`] on malformed
/// arguments.
pub fn parse_command(line: &str) -> Result<UciCommand, ParseCommandError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(ParseCommandError::EmptyLine);
    }

    let (cmd, args) = trimmed
        .split_once(char::is_whitespace)
        .map_or((trimmed, ""), |(c, a)| (c, a.trim_start()));

    match cmd {
        "uci" => Ok(UciCommand::Uci),
        "isready" => Ok(UciCommand::IsReady),
        "ucinewgame" => Ok(UciCommand::UciNewGame),
        "setoption" => parse_setoption(args),
        "quit" => Ok(UciCommand::Quit),
        _ => Err(ParseCommandError::UnknownCommand(cmd.to_string())),
    }
}

fn parse_setoption(args: &str) -> Result<UciCommand, ParseCommandError> {
    let mut tokens = args.split_ascii_whitespace();
    if tokens.next() != Some("name") {
        return Err(ParseCommandError::InvalidSyntax(
            "setoption requires 'name <id>'",
        ));
    }

    let mut name_parts = Vec::new();
    let mut value_parts = Vec::new();
    let mut reading_value = false;

    for tok in tokens {
        if !reading_value && tok == "value" {
            reading_value = true;
            continue;
        }
        if reading_value {
            value_parts.push(tok);
        } else {
            name_parts.push(tok);
        }
    }

    if name_parts.is_empty() {
        return Err(ParseCommandError::InvalidSyntax(
            "setoption option name cannot be empty",
        ));
    }

    let name = name_parts.join(" ");
    let value = reading_value.then(|| value_parts.join(" "));
    Ok(UciCommand::SetOption { name, value })
}
