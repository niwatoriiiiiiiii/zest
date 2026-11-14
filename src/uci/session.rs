use crate::game::Game;
use crate::uci::command::{PositionSpec, UciCommand};

/// Stateful UCI session holding engine components across commands.
#[derive(Debug)]
pub struct UciSession {
    game: Game,
}

impl Default for UciSession {
    fn default() -> Self {
        Self::new()
    }
}

impl UciSession {
    /// Creates a new [`UciSession`] initialized to the Gardner starting position.
    #[must_use]
    pub fn new() -> Self {
        Self { game: Game::new() }
    }

    /// Returns a reference to the active [`Game`].
    #[must_use]
    pub const fn game(&self) -> &Game {
        &self.game
    }

    /// Resets the active game back to the Gardner starting position.
    pub fn reset(&mut self) {
        self.game = Game::new();
    }

    /// Replaces the current position from `spec` and replays `moves` sequentially.
    ///
    /// # Errors
    ///
    /// Returns a human-readable error message if the FEN is invalid or any move in `moves`
    /// is malformed or illegal.
    pub fn apply_position(&mut self, spec: &PositionSpec, moves: &[String]) -> Result<(), String> {
        let mut next_game = match spec {
            PositionSpec::Startpos => Game::new(),
            PositionSpec::Fen(fen) => {
                let normalized = normalize_fen(fen);
                Game::from_fen(&normalized).map_err(|err| err.to_string())?
            }
        };

        for mv in moves {
            next_game.try_uci(mv).map_err(|err| err.to_string())?;
        }

        self.game = next_game;
        Ok(())
    }

    /// Executes a synchronous [`UciCommand`] and returns lines to write to stdout.
    #[must_use]
    pub fn handle_command_sync(&mut self, cmd: &UciCommand) -> Vec<String> {
        match cmd {
            UciCommand::Uci => vec![
                "id name Zest".to_string(),
                "id author niwatoriiiiiiiii".to_string(),
                "option name UCI_Variant type combo default gardner var gardner".to_string(),
                "uciok".to_string(),
            ],
            UciCommand::IsReady => vec!["readyok".to_string()],
            UciCommand::UciNewGame => {
                self.reset();
                Vec::new()
            }
            UciCommand::Position { spec, moves } => match self.apply_position(spec, moves) {
                Ok(()) => Vec::new(),
                Err(err) => vec![format!("info string {err}")],
            },
            UciCommand::Display => format_display(&self.game),
            UciCommand::SetOption { .. } | UciCommand::Quit => Vec::new(),
        }
    }
}

fn normalize_fen(fen: &str) -> String {
    let fields: Vec<&str> = fen.split_ascii_whitespace().collect();
    if fields.len() == 4 {
        format!("{} 0 1", fields.join(" "))
    } else {
        fields.join(" ")
    }
}

fn format_display(game: &Game) -> Vec<String> {
    let board_str = game.board().to_string();
    let game_fen = game.to_fen();
    board_str
        .lines()
        .map(|line| {
            if line.starts_with("Fen: ") {
                format!("Fen: {game_fen}")
            } else {
                line.to_string()
            }
        })
        .collect()
}
