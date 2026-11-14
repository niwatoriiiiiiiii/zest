use crate::core::{Move, PieceType, Square};
use crate::search::types::MATE_THRESHOLD;

const MOVE_VALID_BIT: u16 = 0x8000;
pub(super) const GENERATION_MASK: u8 = 0x3F;
const BOUND_MASK: u8 = 0x03;

/// Score bound classification stored with a transposition table entry.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(super) enum Bound {
    None = 0,
    Exact = 1,
    Lower = 2,
    Upper = 3,
}

impl Bound {
    #[inline]
    pub(super) const fn from_bits(bits: u8) -> Self {
        match bits & BOUND_MASK {
            1 => Self::Exact,
            2 => Self::Lower,
            3 => Self::Upper,
            _ => Self::None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TtEntry {
    pub(super) key: u64,
    pub(super) best_move: u16,
    pub(super) score: i16,
    pub(super) depth: u8,
    pub(super) meta: u8,
}

const _: () = assert!(size_of::<TtEntry>() == 16);

impl TtEntry {
    pub(super) const EMPTY: Self = Self {
        key: 0,
        best_move: 0,
        score: 0,
        depth: 0,
        meta: 0,
    };

    #[inline]
    pub(super) const fn pack_meta(bound: Bound, generation: u8) -> u8 {
        ((generation & GENERATION_MASK) << 2) | (bound as u8 & BOUND_MASK)
    }

    #[inline]
    pub(super) const fn bound(self) -> Bound {
        Bound::from_bits(self.meta)
    }

    #[inline]
    pub(super) const fn generation(self) -> u8 {
        self.meta >> 2
    }
}

#[inline]
pub(super) const fn adjust_mate_score(score: i32, ply: usize, to_tt: bool) -> i32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let ply_i32 = if to_tt { ply as i32 } else { -(ply as i32) };
    if score >= MATE_THRESHOLD {
        score + ply_i32
    } else if score <= -MATE_THRESHOLD {
        score - ply_i32
    } else {
        score
    }
}

#[inline]
pub(super) const fn pack_move(mv: Move) -> u16 {
    let promo = match mv.promotion() {
        Some(PieceType::Knight) => 1,
        Some(PieceType::Bishop) => 2,
        Some(PieceType::Rook) => 3,
        Some(PieceType::Queen) => 4,
        None | Some(PieceType::Pawn | PieceType::King) => 0,
    };
    MOVE_VALID_BIT | (mv.from().as_u8() as u16) | ((mv.to().as_u8() as u16) << 5) | (promo << 10)
}

#[inline]
pub(super) const fn unpack_move(raw: u16) -> Option<Move> {
    if (raw & MOVE_VALID_BIT) == 0 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let (Some(from), Some(to)) = (
        Square::from_index((raw & 0x1F) as u8),
        Square::from_index(((raw >> 5) & 0x1F) as u8),
    ) else {
        return None;
    };
    let promo = match (raw >> 10) & 0x07 {
        0 => None,
        1 => Some(PieceType::Knight),
        2 => Some(PieceType::Bishop),
        3 => Some(PieceType::Rook),
        4 => Some(PieceType::Queen),
        _ => return None,
    };
    Some(Move::new(from, to, promo))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::types::{MATE_SCORE, mate_score_in, mated_score_in};

    #[test]
    fn move_pack_unpack_roundtrip() {
        for promo in [
            None,
            Some(PieceType::Knight),
            Some(PieceType::Bishop),
            Some(PieceType::Rook),
            Some(PieceType::Queen),
        ] {
            let mv = Move::new(Square::B4, Square::E5, promo);
            assert_eq!(unpack_move(pack_move(mv)), Some(mv));
        }
        assert_eq!(unpack_move(0), None);
    }

    #[test]
    fn mate_score_adjustments_roundtrip_across_plies() {
        for ply in [0, 1, 5, 20, 63] {
            let mate = mate_score_in(ply);
            assert_eq!(adjust_mate_score(mate, ply, true), MATE_SCORE);
            assert_eq!(adjust_mate_score(MATE_SCORE, ply, false), mate);

            let mated = mated_score_in(ply);
            assert_eq!(adjust_mate_score(mated, ply, true), -MATE_SCORE);
            assert_eq!(adjust_mate_score(-MATE_SCORE, ply, false), mated);
        }
        assert_eq!(adjust_mate_score(145, 7, true), 145);
        assert_eq!(adjust_mate_score(-230, 12, false), -230);
    }
}
