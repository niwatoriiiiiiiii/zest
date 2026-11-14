use std::fmt::Write as _;

use crate::game::Game;
use crate::search::{SearchResult, is_mate_score, mate_distance_in_moves};

/// Formats engine identification, supported options, and `uciok`.
#[must_use]
pub fn format_uci_id() -> Vec<String> {
    vec![
        "id name Zest".to_string(),
        "id author niwatoriiiiiiiii".to_string(),
        "option name UCI_Variant type combo default gardner var gardner".to_string(),
        "uciok".to_string(),
    ]
}

/// Formats the active position with Stockfish-style board layout and real move clocks.
#[must_use]
pub fn format_display(game: &Game) -> Vec<String> {
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

/// Formats a `SearchResult` as a Stockfish-compatible `info` line.
#[must_use]
pub fn format_info(result: &SearchResult, elapsed_ms: u64) -> String {
    let score_str = if is_mate_score(result.score) {
        mate_distance_in_moves(result.score)
            .map_or_else(|| format!("cp {}", result.score), |m| format!("mate {m}"))
    } else {
        format!("cp {}", result.score)
    };

    let nps = result
        .nodes
        .saturating_mul(1000)
        .checked_div(elapsed_ms)
        .unwrap_or(0);

    let mut line = format!(
        "info depth {} nodes {} nps {} time {} score {}",
        result.depth, result.nodes, nps, elapsed_ms, score_str
    );

    if !result.pv.is_empty() {
        let _ = write!(line, " pv");
        for mv in &result.pv {
            let _ = write!(line, " {mv}");
        }
    }

    line
}

/// Formats the `bestmove` response for a completed [`SearchResult`].
#[must_use]
pub fn format_bestmove(result: &SearchResult) -> String {
    result.best_move.map_or_else(
        || "bestmove (none)".to_string(),
        |mv| format!("bestmove {mv}"),
    )
}
