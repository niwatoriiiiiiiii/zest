mod metrics;
mod types;

use zest::board::Board;
use zest::core::{Color, Move};
use zest::game::{Game, GameStatus};
use zest::movegen::generate_legal_moves;
use zest::search::{SearchLimits, Searcher, is_mate_score, mate_distance_in_moves};

use crate::openings::is_book_prefix;

use self::metrics::{count_safe_candidates, extract_pvs, is_piece_sacrifice, summarize_side};
use self::types::PvCandidate;
pub use self::types::{GameAnalysis, MoveTag, PlyAnalysis, SideSummary};

pub const ANALYSIS_MULTI_PV: usize = 5;
pub const DEFAULT_ANALYSIS_DEPTH: u8 = 12;

fn resolve_played_candidate(
    mv: Move,
    cur_pvs: &[PvCandidate],
    next_pvs: &[PvCandidate],
    status_after: GameStatus,
) -> PvCandidate {
    cur_pvs
        .iter()
        .find(|c| c.mv == mv)
        .copied()
        .unwrap_or_else(|| {
            next_pvs.first().map_or_else(
                || {
                    if matches!(status_after, GameStatus::Checkmate(_)) {
                        PvCandidate {
                            mv,
                            raw_score: 30_000,
                            cp: 10_000,
                            expected: 1.0,
                        }
                    } else {
                        PvCandidate {
                            mv,
                            raw_score: 0,
                            cp: 0,
                            expected: 0.5,
                        }
                    }
                },
                |opp_top| PvCandidate {
                    mv,
                    raw_score: -opp_top.raw_score,
                    cp: -opp_top.cp,
                    expected: 1.0 - opp_top.expected,
                },
            )
        })
}

#[allow(clippy::too_many_arguments)]
fn build_ply_analysis(
    ply: usize,
    side: Color,
    uci: &str,
    mv: Move,
    is_book: bool,
    board_before: &Board,
    board_after: &Board,
    own_legal_count: usize,
    opp_legal: &[Move],
    cur_pvs: &[PvCandidate],
    next_pvs: &[PvCandidate],
    status_after: GameStatus,
) -> Result<PlyAnalysis, String> {
    let top1 = cur_pvs
        .first()
        .copied()
        .ok_or_else(|| format!("ply {ply}: no search PV available"))?;
    let played = resolve_played_candidate(mv, cur_pvs, next_pvs, status_after);

    let d_exp = (top1.expected - played.expected).max(0.0);
    let cp_loss = (top1.cp - played.cp).clamp(0, 1000);

    let own_safe_count = count_safe_candidates(cur_pvs);
    #[allow(clippy::cast_precision_loss)]
    let own_safe_ratio = (own_safe_count as f64) / (own_legal_count.max(1) as f64);
    let own_cp_drop = cur_pvs.get(1).map_or(0, |second| top1.cp - second.cp);

    let opp_legal_count = opp_legal.len();
    let opp_safe_count = count_safe_candidates(next_pvs);
    #[allow(clippy::cast_precision_loss)]
    let opp_safe_ratio = if opp_legal_count == 0 {
        1.0
    } else {
        (opp_safe_count as f64) / (opp_legal_count as f64)
    };
    let opp_drop = match (next_pvs.first(), next_pvs.get(1)) {
        (Some(first), Some(second)) => first.expected - second.expected,
        _ => 0.0,
    };

    let is_best = mv == top1.mv || top1.cp == played.cp;
    let not_losing = played.expected >= 0.20;
    let is_brilliant = d_exp <= 0.015
        && (0.30..=0.90).contains(&played.expected)
        && top1.expected <= 0.90
        && is_piece_sacrifice(board_before, mv, board_after, opp_legal);

    let tag = if is_book {
        MoveTag::Book
    } else if own_legal_count == 1 {
        MoveTag::Forced
    } else if d_exp >= 0.20 {
        MoveTag::Blunder
    } else if d_exp >= 0.10 {
        MoveTag::Mistake
    } else if d_exp >= 0.04 {
        MoveTag::Inaccuracy
    } else if is_brilliant {
        MoveTag::Brilliant
    } else if is_best {
        let sharp_own = own_legal_count >= 4 && own_safe_ratio <= 0.25 && own_cp_drop >= 100;
        let sharp_opp = opp_legal_count >= 4 && opp_safe_ratio <= 0.20 && opp_drop >= 0.10;
        if not_losing && (sharp_own || sharp_opp) {
            MoveTag::Critical
        } else {
            MoveTag::Best
        }
    } else {
        MoveTag::Good
    };

    Ok(PlyAnalysis {
        ply,
        side,
        uci: uci.to_string(),
        score: played.raw_score,
        is_mate: is_mate_score(played.raw_score),
        mate_in: mate_distance_in_moves(played.raw_score),
        expected: played.expected,
        d_exp,
        cp_loss,
        own_safe_count,
        own_legal_count,
        opp_safe_count,
        opp_legal_count,
        tag,
    })
}

/// Analyzes a full game move sequence from `start_fen` using `MultiPV = 5`.
///
/// # Errors
///
/// Returns an error string if `start_fen` is invalid or any move in `moves_uci` is illegal.
pub fn analyze_moves(
    start_fen: &str,
    moves_uci: &[&str],
    depth: u8,
    searcher: &mut Searcher,
) -> Result<GameAnalysis, String> {
    let mut game = Game::from_fen(start_fen).map_err(|e| e.to_string())?;
    let mut still_in_book = *game.board() == Board::gardner_initial();
    let mut prefix_strs: Vec<&str> = Vec::with_capacity(moves_uci.len());
    let limits = SearchLimits::depth(depth).with_multi_pv(ANALYSIS_MULTI_PV);

    let cur_res = searcher.search(game.board(), game.history(), game.halfmove_clock(), limits);
    let mut cur_pvs = extract_pvs(&cur_res);
    let mut plies = Vec::with_capacity(moves_uci.len());

    for (idx, &uci) in moves_uci.iter().enumerate() {
        let board_before = *game.board();
        let side = board_before.side_to_move();
        let own_legal_count = generate_legal_moves(&board_before).len();
        let mv = Move::from_uci(uci).map_err(|e| format!("ply {}: {e}", idx + 1))?;

        prefix_strs.push(uci);
        let is_book = still_in_book && is_book_prefix(&prefix_strs);
        if !is_book {
            still_in_book = false;
        }

        let status_after = game
            .try_move(mv)
            .map_err(|e| format!("ply {} ({uci}): {e}", idx + 1))?;

        let board_after = *game.board();
        let opp_legal = generate_legal_moves(&board_after);
        let next_pvs = if opp_legal.as_slice().is_empty() {
            Vec::new()
        } else {
            let next_res =
                searcher.search(game.board(), game.history(), game.halfmove_clock(), limits);
            extract_pvs(&next_res)
        };

        plies.push(build_ply_analysis(
            idx + 1,
            side,
            uci,
            mv,
            is_book,
            &board_before,
            &board_after,
            own_legal_count,
            opp_legal.as_slice(),
            &cur_pvs,
            &next_pvs,
            status_after,
        )?);
        cur_pvs = next_pvs;
    }

    Ok(GameAnalysis {
        white: summarize_side(&plies, Color::White),
        black: summarize_side(&plies, Color::Black),
        plies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_game_classifications_and_stats() {
        let moves = [
            "a2a3", "b4a3", "b2a3", "e4e3", "d2e3", "e5e4", "e3d4", "b5d4", "b1c3", "e4e5", "c3d5",
            "e5d5", "c2c3", "c5a3", "d1d4",
        ];
        let mut searcher = Searcher::new();
        let start_fen = Board::gardner_initial().to_fen();
        let analysis = analyze_moves(&start_fen, &moves, 8, &mut searcher).expect("analyze");

        assert_eq!(analysis.plies.len(), 15);
        assert_eq!(analysis.plies[0].tag, MoveTag::Book);
        assert_eq!(analysis.plies[3].tag, MoveTag::Book);
        assert_eq!(analysis.plies[5].tag, MoveTag::Blunder);
        assert_eq!(analysis.plies[9].tag, MoveTag::Forced);
        assert!(analysis.white.accuracy > 85.0);
        assert!(analysis.black.accuracy < 65.0);
    }

    #[test]
    fn piece_sacrifice_detects_rook_exposure_on_nb1() {
        let fen = "r1b2/p1kpp/P1n1P/1RPB1/1N2K b - - 2 9";
        let before = Board::from_fen(fen).expect("fen");
        let mv = Move::from_uci("c3b1").expect("mv");
        let mut after = before;
        after.make_move(mv);
        let opp_legal = generate_legal_moves(&after);
        assert!(is_piece_sacrifice(
            &before,
            mv,
            &after,
            opp_legal.as_slice()
        ));
    }
}
