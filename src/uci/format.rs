use std::fmt::Write as _;

use crate::board::Board;
use crate::core::Color;
use crate::eval::{MAX_PHASE, evaluate_breakdown};
use crate::game::Game;
use crate::movegen::generate_legal_moves;
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

/// Formats static evaluation details and final score for `board`.
#[must_use]
pub fn format_eval(board: &Board) -> Vec<String> {
    let b = evaluate_breakdown(board);
    let phase_pct = (b.phase * 100) / MAX_PHASE;
    let side = match board.side_to_move() {
        Color::White => "white side",
        Color::Black => "black side",
    };
    let pawns = f64::from(b.total) / 100.0;

    vec![
        format!("Phase     : {}/{} ({phase_pct}%)", b.phase, MAX_PHASE),
        format!("White     : mg {:+}  eg {:+}", b.white.mg, b.white.eg),
        format!("Black     : mg {:+}  eg {:+}", b.black.mg, b.black.eg),
        format!("Tempo     : {:+} cp", b.tempo),
        String::new(),
        format!(
            "Final evaluation       {pawns:+.2} ({side}, {:+} cp)",
            b.total
        ),
    ]
}

/// Runs perft divide from `board` at `depth` and formats per-move and total node counts.
#[must_use]
pub fn format_perft_divide(board: &Board, depth: usize) -> Vec<String> {
    if depth == 0 {
        return vec![String::new(), "Nodes searched: 1".to_string()];
    }

    let moves = generate_legal_moves(board);
    let mut lines = Vec::with_capacity(moves.len() + 2);
    let mut total = 0u64;

    for &mv in moves.iter() {
        let mut next = *board;
        next.make_move(mv);
        let nodes = perft_nodes(&next, depth - 1);
        total += nodes;
        lines.push(format!("{mv}: {nodes}"));
    }

    lines.push(String::new());
    lines.push(format!("Nodes searched: {total}"));
    lines
}

fn perft_nodes(board: &Board, depth: usize) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = generate_legal_moves(board);
    if depth == 1 {
        return u64::try_from(moves.len()).unwrap_or(0);
    }

    let mut total = 0u64;
    for &mv in moves.iter() {
        let mut next = *board;
        next.make_move(mv);
        total += perft_nodes(&next, depth - 1);
    }
    total
}
