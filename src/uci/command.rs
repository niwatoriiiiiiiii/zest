use std::fmt;

/// Initial board specification for a `position` command.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PositionSpec {
    /// Gardner minichess starting position (`startpos`).
    Startpos,

    /// Explicit Gardner 5x5 FEN string (`fen <fen>`).
    Fen(String),
}

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

    /// Set up the board position and optional move history (`position ...`).
    Position {
        /// Base position (`startpos` or `fen`).
        spec: PositionSpec,

        /// Sequence of UCI moves to replay from the base position.
        moves: Vec<String>,
    },

    /// Display the current board state (`d`).
    Display,

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
        "position" => parse_position(args),
        "d" => Ok(UciCommand::Display),
        "quit" => Ok(UciCommand::Quit),
        _ => Err(ParseCommandError::UnknownCommand(cmd.to_string())),
    }
}

fn parse_position(args: &str) -> Result<UciCommand, ParseCommandError> {
    let tokens: Vec<&str> = args.split_ascii_whitespace().collect();
    let (spec, mut idx) = match tokens.first().copied() {
        Some("startpos") => (PositionSpec::Startpos, 1),
        Some("fen") => {
            let fen_end = tokens[1..]
                .iter()
                .position(|&tok| tok == "moves")
                .map_or(tokens.len(), |offset| 1 + offset);
            if fen_end == 1 {
                return Err(ParseCommandError::InvalidSyntax(
                    "position fen requires a FEN string",
                ));
            }
            (PositionSpec::Fen(tokens[1..fen_end].join(" ")), fen_end)
        }
        _ => {
            return Err(ParseCommandError::InvalidSyntax(
                "position requires 'startpos' or 'fen'",
            ));
        }
    };

    let mut moves = Vec::new();
    if let Some(&next) = tokens.get(idx) {
        if next != "moves" {
            return Err(ParseCommandError::InvalidSyntax(
                "expected 'moves' after position specification",
            ));
        }
        idx += 1;
        moves.extend(tokens[idx..].iter().map(|&mv| mv.to_string()));
    }

    Ok(UciCommand::Position { spec, moves })
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
