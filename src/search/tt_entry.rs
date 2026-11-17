use crate::core::{Move, PieceType, Square};
use crate::search::types::MATE_THRESHOLD;

const MOVE_VALID_BIT: u16 = 0x8000;
pub(super) const GENERATION_MASK: u8 = 0x3F;
const BOUND_MASK: u8 = 0x03;
pub(super) const EVAL_NONE: i16 = i16::MIN;

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

/// Decoded transposition table lookup result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct TtHit {
    pub(super) best_move: Option<Move>,
    pub(super) score: i32,
    pub(super) eval: Option<i32>,
    pub(super) depth: u8,
    pub(super) bound: Bound,
}

/// Outcome of probing the TT against a search window `(alpha, beta)`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct TtProbe {
    pub(super) cutoff: Option<(i32, bool)>,
    pub(super) best_move: Option<Move>,
    pub(super) static_eval: Option<i32>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TtEntry {
    pub(super) key: u64,
    pub(super) best_move: u16,
    pub(super) score: i16,
    pub(super) eval: i16,
    pub(super) depth: u8,
    pub(super) meta: u8,
}

const _: () = assert!(size_of::<TtEntry>() == 16);

impl TtEntry {
    pub(super) const EMPTY: Self = Self {
        key: 0,
        best_move: 0,
        score: 0,
        eval: EVAL_NONE,
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

    #[inline]
    pub(super) const fn replacement_age(self, generation: u8) -> u16 {
        let stale = if self.generation() == generation {
            0
        } else {
            256
        };
        stale + (u8::MAX - self.depth) as u16
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn write_or_fill(
        &mut self,
        key: u64,
        depth: u8,
        bound: Bound,
        score: i32,
        packed_move: u16,
        packed_eval: i16,
        generation: u8,
    ) {
        let existing_bound = self.bound();
        let same_key = existing_bound != Bound::None && self.key == key;
        let should_replace = existing_bound == Bound::None
            || if same_key {
                depth >= self.depth || (bound == Bound::Exact && existing_bound != Bound::Exact)
            } else {
                self.generation() != generation || depth >= self.depth
            };

        if should_replace {
            let final_eval = if packed_eval != EVAL_NONE {
                packed_eval
            } else if same_key {
                self.eval
            } else {
                EVAL_NONE
            };
            #[allow(clippy::cast_possible_truncation)]
            let clamped_score = score.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            *self = Self {
                key,
                best_move: packed_move,
                score: clamped_score,
                eval: final_eval,
                depth,
                meta: Self::pack_meta(bound, generation),
            };
        } else if same_key {
            if self.best_move == 0 && packed_move != 0 {
                self.best_move = packed_move;
            }
            if self.eval == EVAL_NONE && packed_eval != EVAL_NONE {
                self.eval = packed_eval;
            }
        }
    }
}

#[inline]
pub(super) const fn pack_eval(static_eval: Option<i32>) -> i16 {
    match static_eval {
        Some(eval) => {
            let min_v = (EVAL_NONE as i32) + 1;
            let max_v = i16::MAX as i32;
            #[allow(clippy::cast_possible_truncation)]
            if eval < min_v {
                min_v as i16
            } else if eval > max_v {
                max_v as i16
            } else {
                eval as i16
            }
        }
        None => EVAL_NONE,
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
