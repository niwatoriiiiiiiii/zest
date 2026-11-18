use zest::board::Board;
use zest::core::{Color, Move, PieceType, Square};
use zest::game::{Game, GameStatus};
use zest::movegen::{generate_legal_moves, is_square_attacked};
use zest::search::{
    SearchLimits, SearchResult, Searcher, Wdl, is_mate_score, mate_distance_in_moves,
};

use crate::openings::is_book_prefix;

pub const ANALYSIS_MULTI_PV: usize = 5;
pub const DEFAULT_ANALYSIS_DEPTH: u8 = 12;

/// Move classification tags tailored for 5x5 Gardner minichess.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveTag {
    Book,
    Forced,
    Blunder,
    Mistake,
    Inaccuracy,
    Brilliant,
    Critical,
    Best,
    Good,
}

impl MoveTag {
    /// Returns the display name of the classification tag.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Book => "Book",
            Self::Forced => "Forced",
            Self::Blunder => "Blunder",
            Self::Mistake => "Mistake",
            Self::Inaccuracy => "Inaccuracy",
            Self::Brilliant => "Brilliant",
            Self::Critical => "Critical",
            Self::Best => "Best",
            Self::Good => "Good",
        }
    }

    /// Returns the glyph symbol for UI annotation.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Book => "📖",
            Self::Forced => "□",
            Self::Blunder => "??",
            Self::Mistake => "?",
            Self::Inaccuracy => "?!",
            Self::Brilliant => "!!",
            Self::Critical => "!",
            Self::Best => "✦",
            Self::Good => "○",
        }
    }

    /// Returns the standard PGN Numeric Annotation Glyph (NAG), if defined.
    #[must_use]
    pub const fn nag(self) -> Option<u8> {
        match self {
            Self::Critical => Some(1),
            Self::Mistake => Some(2),
            Self::Brilliant => Some(3),
            Self::Blunder => Some(4),
            Self::Inaccuracy => Some(6),
            Self::Forced => Some(7),
            Self::Book | Self::Best | Self::Good => None,
        }
    }

    /// Returns `true` if this move should be excluded from Accuracy and ACPL aggregation.
    #[must_use]
    pub const fn is_excluded_from_stats(self) -> bool {
        matches!(self, Self::Book | Self::Forced)
    }
}

/// Evaluation summary for a single candidate PV line.
#[derive(Clone, Copy, Debug)]
struct PvCandidate {
    mv: Move,
    raw_score: i32,
    cp: i32,
    expected: f64,
}

/// Classification and metrics for a single played move.
#[derive(Clone, Debug)]
pub struct PlyAnalysis {
    pub ply: usize,
    pub side: Color,
    pub uci: String,
    pub score: i32,
    pub is_mate: bool,
    pub mate_in: Option<i32>,
    pub expected: f64,
    pub d_exp: f64,
    pub cp_loss: i32,
    pub own_safe_count: usize,
    pub own_legal_count: usize,
    pub opp_safe_count: usize,
    pub opp_legal_count: usize,
    pub tag: MoveTag,
}

/// Aggregated accuracy and classification counts for one side.
#[derive(Clone, Debug)]
pub struct SideSummary {
    pub accuracy: f64,
    pub acpl: f64,
    pub rms_d_exp: f64,
    pub counted_moves: usize,
    pub book: usize,
    pub forced: usize,
    pub brilliant: usize,
    pub critical: usize,
    pub best: usize,
    pub good: usize,
    pub inaccuracy: usize,
    pub mistake: usize,
    pub blunder: usize,
}

/// Full post-game analysis result for both sides.
#[derive(Clone, Debug)]
pub struct GameAnalysis {
    pub plies: Vec<PlyAnalysis>,
    pub white: SideSummary,
    pub black: SideSummary,
}

const fn piece_value(pt: PieceType) -> i32 {
    match pt {
        PieceType::Pawn => 1,
        PieceType::Knight | PieceType::Bishop => 3,
        PieceType::Rook => 5,
        PieceType::Queen => 9,
        PieceType::King => 100,
    }
}

fn score_to_cp(score: i32) -> i32 {
    if is_mate_score(score) {
        let dist = mate_distance_in_moves(score).unwrap_or(1).abs();
        if score > 0 {
            10_000 - dist
        } else {
            -10_000 + dist
        }
    } else {
        score
    }
}

fn extract_pvs(res: &SearchResult) -> Vec<PvCandidate> {
    res.pvs
        .iter()
        .filter_map(|line| {
            let mv = *line.moves.first()?;
            let expected = Wdl::from_score(line.score, res.phase).expected_score();
            Some(PvCandidate {
                mv,
                raw_score: line.score,
                cp: score_to_cp(line.score),
                expected,
            })
        })
        .collect()
}

fn count_safe_candidates(cands: &[PvCandidate]) -> usize {
    let Some(top) = cands.first() else {
        return 0;
    };
    cands
        .iter()
        .filter(|v| (top.expected - v.expected) <= 0.05 && (top.cp - v.cp).abs() <= 80)
        .count()
}

/// Returns `true` if `mv` leaves a minor piece or higher exposed to a net material loss.
#[must_use]
pub fn is_piece_sacrifice(
    board_before: &Board,
    mv: Move,
    board_after: &Board,
    opp_legal: &[Move],
) -> bool {
    let mover = board_before.side_to_move();
    let captured_val = board_before
        .piece_at(mv.to())
        .map_or(0, |p| piece_value(p.piece_type));

    let mut opp_min_att = [i32::MAX; 25];
    for &om in opp_legal {
        if let Some(att) = board_after.piece_at(om.from()) {
            let av = piece_value(att.piece_type);
            let idx = om.to().index();
            if av < opp_min_att[idx] {
                opp_min_att[idx] = av;
            }
        }
    }

    for idx in 0..25_u8 {
        let Some(sq) = Square::from_index(idx) else {
            continue;
        };
        let Some(piece) = board_after.piece_at(sq) else {
            continue;
        };
        if piece.color != mover {
            continue;
        }
        let val = piece_value(piece.piece_type);
        let att_val = opp_min_att[usize::from(idx)];
        if val < 3 || att_val == i32::MAX {
            continue;
        }
        let defended = is_square_attacked(board_after, sq, mover);
        if (!defended && val > captured_val) || (defended && (val - att_val) > captured_val) {
            return true;
        }
    }
    false
}

fn summarize_side(plies: &[PlyAnalysis], side: Color) -> SideSummary {
    let mut sum_sq_dexp = 0.0_f64;
    let mut sum_cp_loss = 0_i64;
    let mut counted = 0_usize;

    let mut book = 0;
    let mut forced = 0;
    let mut brilliant = 0;
    let mut critical = 0;
    let mut best = 0;
    let mut good = 0;
    let mut inaccuracy = 0;
    let mut mistake = 0;
    let mut blunder = 0;

    for p in plies.iter().filter(|p| p.side == side) {
        match p.tag {
            MoveTag::Book => book += 1,
            MoveTag::Forced => forced += 1,
            MoveTag::Brilliant => brilliant += 1,
            MoveTag::Critical => critical += 1,
            MoveTag::Best => best += 1,
            MoveTag::Good => good += 1,
            MoveTag::Inaccuracy => inaccuracy += 1,
            MoveTag::Mistake => mistake += 1,
            MoveTag::Blunder => blunder += 1,
        }
        if !p.tag.is_excluded_from_stats() {
            counted += 1;
            sum_sq_dexp = p.d_exp.mul_add(p.d_exp, sum_sq_dexp);
            sum_cp_loss += i64::from(p.cp_loss);
        }
    }

    let (accuracy, acpl, rms_d_exp) = if counted > 0 {
        #[allow(clippy::cast_precision_loss)]
        let n = counted as f64;
        let rms = (sum_sq_dexp / n).sqrt();
        let acc = 100.0 * (-3.5 * rms).exp();
        #[allow(clippy::cast_precision_loss)]
        let avg_cpl = (sum_cp_loss as f64) / n;
        (acc, avg_cpl, rms)
    } else {
        (100.0, 0.0, 0.0)
    };

    SideSummary {
        accuracy,
        acpl,
        rms_d_exp,
        counted_moves: counted,
        book,
        forced,
        brilliant,
        critical,
        best,
        good,
        inaccuracy,
        mistake,
        blunder,
    }
}

/// Analyzes a full game move sequence from `start_fen` using `MultiPV = 5`.
///
/// # Errors
///
/// Returns an error string if `start_fen` is invalid or any move in `moves_uci` is illegal.
#[allow(clippy::too_many_lines)]
pub fn analyze_moves(
    start_fen: &str,
    moves_uci: &[&str],
    depth: u8,
    searcher: &mut Searcher,
) -> Result<GameAnalysis, String> {
    let mut game = Game::from_fen(start_fen).map_err(|e| e.to_string())?;
    let initial_board = *game.board();
    let check_book = initial_board == Board::gardner_initial();
    let mut still_in_book = check_book;
    let mut prefix_strs: Vec<&str> = Vec::with_capacity(moves_uci.len());
    let limits = SearchLimits::depth(depth).with_multi_pv(ANALYSIS_MULTI_PV);

    let cur_res = searcher.search(game.board(), game.history(), game.halfmove_clock(), limits);
    let mut cur_pvs = extract_pvs(&cur_res);
    let mut plies = Vec::with_capacity(moves_uci.len());

    for (idx, &uci) in moves_uci.iter().enumerate() {
        let board_before = *game.board();
        let side = board_before.side_to_move();
        let legal_before = generate_legal_moves(&board_before);
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
        let opp_legal_slice = opp_legal.as_slice();

        let next_pvs = if opp_legal_slice.is_empty() {
            Vec::new()
        } else {
            let next_res =
                searcher.search(game.board(), game.history(), game.halfmove_clock(), limits);
            extract_pvs(&next_res)
        };

        let top1 = cur_pvs
            .first()
            .copied()
            .ok_or_else(|| format!("ply {}: no search PV available", idx + 1))?;

        let played = cur_pvs
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
            });

        let d_exp = (top1.expected - played.expected).max(0.0);
        let cp_loss = (top1.cp - played.cp).clamp(0, 1000);

        let own_legal_count = legal_before.len();
        let own_safe_count = count_safe_candidates(&cur_pvs);
        #[allow(clippy::cast_precision_loss)]
        let own_safe_ratio = (own_safe_count as f64) / (own_legal_count.max(1) as f64);
        let own_cp_drop = cur_pvs.get(1).map_or(0, |second| top1.cp - second.cp);

        let opp_legal_count = opp_legal_slice.len();
        let opp_safe_count = count_safe_candidates(&next_pvs);
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
            && is_piece_sacrifice(&board_before, mv, &board_after, opp_legal_slice);

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

        plies.push(PlyAnalysis {
            ply: idx + 1,
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
        });

        cur_pvs = next_pvs;
    }

    let white = summarize_side(&plies, Color::White);
    let black = summarize_side(&plies, Color::Black);

    Ok(GameAnalysis {
        plies,
        white,
        black,
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
