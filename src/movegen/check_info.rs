use crate::board::{Bitboard, Board};
use crate::core::{Color, PieceType, Square};
use crate::movegen::{between, bishop_attacks, knight_attacks, pawn_attacks, rook_attacks};

/// Precomputed check and pin information for the player to move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckInfo {
    /// Enemy pieces directly giving check to our king.
    pub checkers: Bitboard,

    /// Friendly pieces pinned to our king.
    pub pinned: Bitboard,

    /// For each pinned piece index, the legal ray mask.
    pub pin_rays: [Bitboard; 25],
}

impl CheckInfo {
    /// Creates an empty `CheckInfo`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            checkers: Bitboard::EMPTY,
            pinned: Bitboard::EMPTY,
            pin_rays: [Bitboard::EMPTY; 25],
        }
    }

    /// Computes check and pin information for `us` on the given `board`.
    #[must_use]
    pub fn compute(board: &Board, us: Color, king_sq: Square) -> Self {
        let mut info = Self::new();
        let them = !us;
        let occupied = board.occupied();
        let friendly = board.color_bb(us);
        let enemies = board.color_bb(them);

        // Direct checks from knights and pawns
        info.checkers =
            (knight_attacks(king_sq) & enemies & board.piece_type_bb(PieceType::Knight))
                | (pawn_attacks(king_sq, us) & enemies & board.piece_type_bb(PieceType::Pawn));

        // Slider checks and pins (orthogonal and diagonal)
        let queens = board.piece_type_bb(PieceType::Queen);
        let rook_sliders = enemies & (board.piece_type_bb(PieceType::Rook) | queens);
        let bishop_sliders = enemies & (board.piece_type_bb(PieceType::Bishop) | queens);

        let mut slider_candidates = (rook_attacks(king_sq, Bitboard::EMPTY) & rook_sliders)
            | (bishop_attacks(king_sq, Bitboard::EMPTY) & bishop_sliders);

        while let Some(s) = slider_candidates.pop_lsb() {
            let between_bb = between(king_sq, s);
            let mut blockers = between_bb & occupied;
            match blockers.count() {
                0 => info.checkers.set(s),
                1 => {
                    if let Some(blocker_sq) = blockers.pop_lsb()
                        && friendly.contains(blocker_sq)
                    {
                        info.pinned.set(blocker_sq);
                        info.pin_rays[blocker_sq.index()] = between_bb | Bitboard::from_square(s);
                    }
                }
                _ => {}
            }
        }

        info
    }
}

impl Default for CheckInfo {
    fn default() -> Self {
        Self::new()
    }
}
