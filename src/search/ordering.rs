use crate::board::Board;
use crate::core::{Move, PieceType, Square};
use crate::movegen::{MAX_MOVES, MoveList};
use crate::search::state::SearchState;

const DUMMY_MOVE: Move = Move::new(Square::A1, Square::A1, None);
const PRIORITY_MOVE_SCORE: i32 = 1_000_000;
const CAPTURE_BASE_SCORE: i32 = 10_000;
const PROMO_BASE_SCORE: i32 = 2_000;

#[derive(Clone, Copy)]
pub(super) struct ScoredMove {
    mv: Move,
    score: i32,
}

impl ScoredMove {
    pub(super) const EMPTY: Self = Self {
        mv: DUMMY_MOVE,
        score: 0,
    };

    #[inline]
    pub(super) const fn mv(self) -> Move {
        self.mv
    }
}

#[inline]
const fn mvv_value(pt: PieceType) -> i32 {
    match pt {
        PieceType::Pawn => 100,
        PieceType::Bishop => 230,
        PieceType::Knight => 240,
        PieceType::Rook => 315,
        PieceType::Queen => 550,
        PieceType::King => 10_000,
    }
}

#[inline]
const fn lva_rank(pt: PieceType) -> i32 {
    match pt {
        PieceType::Pawn => 1,
        PieceType::Bishop | PieceType::Knight => 2,
        PieceType::Rook => 3,
        PieceType::Queen => 4,
        PieceType::King => 5,
    }
}

#[inline]
const fn promo_bonus(pt: PieceType) -> i32 {
    match pt {
        PieceType::Queen => PROMO_BASE_SCORE + 550,
        PieceType::Rook => PROMO_BASE_SCORE + 315,
        PieceType::Knight => PROMO_BASE_SCORE + 240,
        PieceType::Bishop => PROMO_BASE_SCORE + 230,
        PieceType::Pawn | PieceType::King => 0,
    }
}

#[inline]
fn score_tactical_or_priority(board: &Board, mv: Move, priority_move: Option<Move>) -> Option<i32> {
    if priority_move == Some(mv) {
        return Some(PRIORITY_MOVE_SCORE);
    }

    let capture_score = board.piece_at(mv.to()).map(|victim| {
        let attacker_rank = board
            .piece_at(mv.from())
            .map_or(6, |p| lva_rank(p.piece_type));
        CAPTURE_BASE_SCORE + mvv_value(victim.piece_type) * 16 - attacker_rank
    });
    let promo_score = mv.promotion().map(promo_bonus);
    match (capture_score, promo_score) {
        (Some(c), Some(p)) => Some(c + p),
        (Some(c), None) => Some(c),
        (None, Some(p)) => Some(p),
        (None, None) => None,
    }
}

#[inline]
pub(super) fn order_moves(
    board: &Board,
    moves: &MoveList,
    priority_move: Option<Move>,
    state: &SearchState,
    ply: usize,
) -> ([ScoredMove; MAX_MOVES], usize) {
    let mut out = [ScoredMove::EMPTY; MAX_MOVES];
    let len = moves.len();
    let side = board.side_to_move();
    for (dst, &mv) in out[..len].iter_mut().zip(moves.iter()) {
        let score = score_tactical_or_priority(board, mv, priority_move)
            .unwrap_or_else(|| state.quiet_move_score(side, ply, mv));
        *dst = ScoredMove { mv, score };
    }
    (out, len)
}

#[inline]
pub(super) fn order_tactical_moves(
    board: &Board,
    moves: &MoveList,
    priority_move: Option<Move>,
) -> ([ScoredMove; MAX_MOVES], usize) {
    let mut out = [ScoredMove::EMPTY; MAX_MOVES];
    let mut len = 0;
    for &mv in moves.iter() {
        let is_capture = board.piece_at(mv.to()).is_some();
        if is_capture || mv.promotion().is_some() {
            let score = score_tactical_or_priority(board, mv, priority_move).unwrap_or(0);
            out[len] = ScoredMove { mv, score };
            len += 1;
        }
    }
    (out, len)
}

#[inline]
pub(super) fn pick_next_move(
    scored: &mut [ScoredMove; MAX_MOVES],
    start: usize,
    len: usize,
) -> Move {
    let mut best_idx = start;
    let mut best_score = scored[start].score;
    for (i, item) in scored.iter().enumerate().take(len).skip(start + 1) {
        if item.score > best_score {
            best_score = item.score;
            best_idx = i;
        }
    }
    scored.swap(start, best_idx);
    scored[start].mv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::state::HISTORY_MAX;

    #[test]
    fn ordering_bands_keep_priority_tactical_killer_and_history_ordered() {
        let board = Board::from_fen("4k/1P3/3n1/5/1Q1K1 w - - 0 1").expect("valid FEN");
        let moves = generate_moves(&board);
        let priority_mv = Move::from_uci("d1e2").expect("valid move");
        let capture_mv = Move::from_uci("b1d3").expect("valid move");
        let promo_mv = Move::from_uci("b4b5q").expect("valid move");
        let killer1_mv = Move::from_uci("b1a1").expect("valid move");
        let killer2_mv = Move::from_uci("b1c1").expect("valid move");
        let history_mv = Move::from_uci("b1b2").expect("valid move");
        let plain_mv = Move::from_uci("b1a2").expect("valid move");

        let mut state = SearchState::new();
        state.record_killer(2, killer2_mv);
        state.record_killer(2, killer1_mv);
        state.update_history(board.side_to_move(), history_mv, 128);

        let (mut scored, len) = order_moves(&board, &moves, Some(priority_mv), &state, 2);
        let mut ranked = Vec::with_capacity(len);
        for idx in 0..len {
            ranked.push(pick_next_move(&mut scored, idx, len));
        }

        let pos = |mv: Move| {
            ranked
                .iter()
                .position(|&candidate| candidate == mv)
                .expect("expected legal move in ordering output")
        };
        assert!(pos(priority_mv) < pos(capture_mv));
        assert!(pos(capture_mv) < pos(promo_mv));
        assert!(pos(promo_mv) < pos(killer1_mv));
        assert!(pos(killer1_mv) < pos(killer2_mv));
        assert!(pos(killer2_mv) < pos(history_mv));
        assert!(pos(history_mv) < pos(plain_mv));
    }

    fn generate_moves(board: &Board) -> MoveList {
        crate::movegen::generate_legal_moves(board)
    }

    #[test]
    fn history_gravity_and_killer_slots_stay_bounded_and_reset() {
        let mut state = SearchState::new();
        let mv1 = Move::from_uci("a2a3").expect("valid move");
        let mv2 = Move::from_uci("b2b3").expect("valid move");

        state.record_killer(1, mv1);
        state.record_killer(1, mv1);
        state.record_killer(1, mv2);
        assert_ne!(
            state.quiet_move_score(crate::core::Color::White, 1, mv2),
            state.quiet_move_score(crate::core::Color::White, 1, mv1)
        );

        for _ in 0..64 {
            state.update_history(crate::core::Color::White, mv1, HISTORY_MAX);
            state.update_history(crate::core::Color::White, mv2, -HISTORY_MAX);
        }
        assert_eq!(
            state.quiet_move_score(crate::core::Color::White, 0, mv1),
            HISTORY_MAX * 2
        );
        assert_eq!(state.quiet_move_score(crate::core::Color::White, 0, mv2), 0);

        state.clear_heuristics();
        assert_eq!(
            state.quiet_move_score(crate::core::Color::White, 1, mv1),
            HISTORY_MAX
        );
        assert_eq!(
            state.quiet_move_score(crate::core::Color::White, 1, mv2),
            HISTORY_MAX
        );
    }
}
