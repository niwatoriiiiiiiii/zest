use crate::board::{Bitboard, Board};
use crate::core::{Color, Piece, PieceType, Square};
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

        // Direct checks from knights
        let mut knight_threats = knight_attacks(king_sq);
        while let Some(s) = knight_threats.pop_lsb() {
            if board.piece_at(s) == Some(Piece::new(them, PieceType::Knight)) {
                info.checkers.set(s);
            }
        }

        // Direct checks from pawns
        let mut pawn_threats = pawn_attacks(king_sq, us);
        while let Some(s) = pawn_threats.pop_lsb() {
            if board.piece_at(s) == Some(Piece::new(them, PieceType::Pawn)) {
                info.checkers.set(s);
            }
        }

        // Slider checks and pins (orthogonal: Rook / Queen)
        let mut rook_candidates = rook_attacks(king_sq, Bitboard::EMPTY);
        while let Some(s) = rook_candidates.pop_lsb() {
            if let Some(piece) = board.piece_at(s)
                && piece.color == them
                && (piece.piece_type == PieceType::Rook || piece.piece_type == PieceType::Queen)
            {
                let between_bb = between(king_sq, s);
                let blockers = between_bb & occupied;
                match blockers.count() {
                    0 => info.checkers.set(s),
                    1 => {
                        let mut b_copy = blockers;
                        if let Some(blocker_sq) = b_copy.pop_lsb()
                            && let Some(blocker_piece) = board.piece_at(blocker_sq)
                            && blocker_piece.color == us
                        {
                            info.pinned.set(blocker_sq);
                            info.pin_rays[blocker_sq.index()] =
                                between_bb | Bitboard::from_square(s);
                        }
                    }
                    _ => {}
                }
            }
        }

        // Slider checks and pins (diagonal: Bishop / Queen)
        let mut bishop_candidates = bishop_attacks(king_sq, Bitboard::EMPTY);
        while let Some(s) = bishop_candidates.pop_lsb() {
            if let Some(piece) = board.piece_at(s)
                && piece.color == them
                && (piece.piece_type == PieceType::Bishop || piece.piece_type == PieceType::Queen)
            {
                let between_bb = between(king_sq, s);
                let blockers = between_bb & occupied;
                match blockers.count() {
                    0 => info.checkers.set(s),
                    1 => {
                        let mut b_copy = blockers;
                        if let Some(blocker_sq) = b_copy.pop_lsb()
                            && let Some(blocker_piece) = board.piece_at(blocker_sq)
                            && blocker_piece.color == us
                        {
                            info.pinned.set(blocker_sq);
                            info.pin_rays[blocker_sq.index()] =
                                between_bb | Bitboard::from_square(s);
                        }
                    }
                    _ => {}
                }
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
