use crate::board::Board;
use crate::core::{Move, PieceType, Square};
use crate::movegen::{MAX_MOVES, MoveList};

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
pub(super) fn score_move(board: &Board, mv: Move, priority_move: Option<Move>) -> i32 {
    if priority_move == Some(mv) {
        return PRIORITY_MOVE_SCORE;
    }

    let mut score = 0;
    if let Some(victim) = board.piece_at(mv.to()) {
        let attacker_rank = board
            .piece_at(mv.from())
            .map_or(6, |p| lva_rank(p.piece_type));
        score += CAPTURE_BASE_SCORE + mvv_value(victim.piece_type) * 16 - attacker_rank;
    }
    if let Some(promo) = mv.promotion() {
        score += promo_bonus(promo);
    }
    score
}

#[inline]
pub(super) fn order_moves(
    board: &Board,
    moves: &MoveList,
    priority_move: Option<Move>,
) -> ([ScoredMove; MAX_MOVES], usize) {
    let mut out = [ScoredMove::EMPTY; MAX_MOVES];
    let len = moves.len();
    for (dst, &mv) in out[..len].iter_mut().zip(moves.iter()) {
        *dst = ScoredMove {
            mv,
            score: score_move(board, mv, priority_move),
        };
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
            out[len] = ScoredMove {
                mv,
                score: score_move(board, mv, priority_move),
            };
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
