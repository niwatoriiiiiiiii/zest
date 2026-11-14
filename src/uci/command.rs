use std::fmt;

/// Initial board specification for a `position` command.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PositionSpec {
    /// Gardner minichess starting position (`startpos`).
    Startpos,

    /// Explicit Gardner 5x5 FEN string (`fen <fen>`).
    Fen(String),
}

/// Parameters supplied to a `go` search command.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct GoParams {
    /// Maximum iterative-deepening depth (`depth <n>`).
    pub depth: Option<u8>,

    /// Maximum visited nodes (`nodes <n>`).
    pub nodes: Option<u64>,

    /// Fixed search time in milliseconds (`movetime <ms>`).
    pub movetime: Option<u64>,

    /// White remaining clock in milliseconds (`wtime <ms>`).
    pub wtime: Option<u64>,

    /// Black remaining clock in milliseconds (`btime <ms>`).
    pub btime: Option<u64>,

    /// White time increment per move in milliseconds (`winc <ms>`).
    pub winc: Option<u64>,

    /// Black time increment per move in milliseconds (`binc <ms>`).
    pub binc: Option<u64>,

    /// Moves remaining until next time control (`movestogo <n>`).
    pub movestogo: Option<u16>,

    /// Search until an explicit `stop` command (`infinite`).
    pub infinite: bool,
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

    /// Start searching the current position (`go ...`).
    Go(GoParams),

    /// Run a perft divide count on the current position (`go perft <depth>`).
    GoPerft(usize),

    /// Display static evaluation details for the current position (`eval`).
    Eval,

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
        "go" => parse_go(args),
        "eval" => Ok(UciCommand::Eval),
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

fn parse_next_int<T: std::str::FromStr>(
    tokens: &mut std::str::SplitAsciiWhitespace<'_>,
    err_msg: &'static str,
) -> Result<T, ParseCommandError> {
    let raw = tokens
        .next()
        .ok_or(ParseCommandError::InvalidSyntax(err_msg))?;
    raw.parse::<T>()
        .map_err(|_| ParseCommandError::InvalidSyntax(err_msg))
}

fn parse_go(args: &str) -> Result<UciCommand, ParseCommandError> {
    let mut tokens = args.split_ascii_whitespace();
    if tokens.clone().next() == Some("perft") {
        tokens.next();
        let depth = parse_next_int(&mut tokens, "invalid 'go perft' depth")?;
        if tokens.next().is_some() {
            return Err(ParseCommandError::InvalidSyntax(
                "unexpected extra arguments after 'go perft <depth>'",
            ));
        }
        return Ok(UciCommand::GoPerft(depth));
    }

    let mut params = GoParams::default();

    while let Some(tok) = tokens.next() {
        match tok {
            "depth" => {
                params.depth = Some(parse_next_int(&mut tokens, "invalid 'go depth' value")?);
            }
            "nodes" => {
                params.nodes = Some(parse_next_int(&mut tokens, "invalid 'go nodes' value")?);
            }
            "movetime" => {
                params.movetime = Some(parse_next_int(&mut tokens, "invalid 'go movetime' value")?);
            }
            "wtime" => {
                params.wtime = Some(parse_next_int(&mut tokens, "invalid 'go wtime' value")?);
            }
            "btime" => {
                params.btime = Some(parse_next_int(&mut tokens, "invalid 'go btime' value")?);
            }
            "winc" => {
                params.winc = Some(parse_next_int(&mut tokens, "invalid 'go winc' value")?);
            }
            "binc" => {
                params.binc = Some(parse_next_int(&mut tokens, "invalid 'go binc' value")?);
            }
            "movestogo" => {
                params.movestogo =
                    Some(parse_next_int(&mut tokens, "invalid 'go movestogo' value")?);
            }
            "infinite" => params.infinite = true,
            _ => {}
        }
    }

    Ok(UciCommand::Go(params))
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
