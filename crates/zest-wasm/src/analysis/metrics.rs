use zest::board::Board;
use zest::core::{Color, Move, PieceType, Square};
use zest::movegen::is_square_attacked;
use zest::search::{SearchResult, Wdl, is_mate_score, mate_distance_in_moves};

use super::types::{MoveTag, PlyAnalysis, PvCandidate, SideSummary};

pub const fn piece_value(pt: PieceType) -> i32 {
    match pt {
        PieceType::Pawn => 1,
        PieceType::Knight | PieceType::Bishop => 3,
        PieceType::Rook => 5,
        PieceType::Queen => 9,
        PieceType::King => 100,
    }
}

pub fn score_to_cp(score: i32) -> i32 {
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

pub fn extract_pvs(res: &SearchResult) -> Vec<PvCandidate> {
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

pub fn count_safe_candidates(cands: &[PvCandidate]) -> usize {
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

pub fn summarize_side(plies: &[PlyAnalysis], side: Color) -> SideSummary {
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
