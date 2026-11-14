use core::sync::atomic::AtomicBool;

use crate::game::Game;
use crate::search::{SearchResult, Searcher};
use crate::uci::command::{GoParams, PositionSpec, UciCommand};
use crate::uci::format::{
    format_bestmove, format_display, format_eval, format_info, format_perft_divide, format_uci_id,
};
use crate::uci::time::compute_search_limits;

/// Stateful UCI session holding engine components across commands.
#[derive(Debug)]
pub struct UciSession {
    game: Game,
    searcher: Option<Searcher>,
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
        Self {
            game: Game::new(),
            searcher: Some(Searcher::new()),
        }
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

    /// Takes ownership of the reusable [`Searcher`] for background execution.
    #[must_use]
    pub fn take_searcher(&mut self) -> Searcher {
        self.searcher.take().unwrap_or_default()
    }

    /// Returns a previously taken [`Searcher`] back to the session for reuse.
    pub fn restore_searcher(&mut self, searcher: Searcher) {
        self.searcher = Some(searcher);
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

    /// Executes a synchronous search for `params` on the current game state.
    #[must_use]
    pub fn search_sync(&mut self, params: &GoParams) -> SearchResult {
        let never_stop = AtomicBool::new(false);
        self.search_with_stop(params, &never_stop)
    }

    /// Executes a search for `params` while polling `stop` for early termination.
    #[must_use]
    pub fn search_with_stop(&mut self, params: &GoParams, stop: &AtomicBool) -> SearchResult {
        let limits = compute_search_limits(params);
        let searcher = self.searcher.get_or_insert_with(Searcher::new);
        searcher.search_with_stop(
            self.game.board(),
            self.game.history(),
            self.game.halfmove_clock(),
            limits,
            stop,
        )
    }

    /// Executes a synchronous [`UciCommand`] and returns lines to write to stdout.
    #[must_use]
    pub fn handle_command_sync(&mut self, cmd: &UciCommand) -> Vec<String> {
        match cmd {
            UciCommand::Uci => format_uci_id(),
            UciCommand::IsReady => vec!["readyok".to_string()],
            UciCommand::UciNewGame => {
                self.reset();
                Vec::new()
            }
            UciCommand::Position { spec, moves } => match self.apply_position(spec, moves) {
                Ok(()) => Vec::new(),
                Err(err) => vec![format!("info string {err}")],
            },
            UciCommand::Go(params) => {
                let result = self.search_sync(params);
                vec![format_info(&result, 0), format_bestmove(&result)]
            }
            UciCommand::GoPerft(depth) => format_perft_divide(self.game.board(), *depth),
            UciCommand::Eval => format_eval(self.game.board()),
            UciCommand::Display => format_display(&self.game),
            UciCommand::SetOption { .. } | UciCommand::Stop | UciCommand::Quit => Vec::new(),
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
