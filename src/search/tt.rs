use crate::core::{Move, PieceType, Square};
use crate::search::types::MATE_THRESHOLD;

/// Default transposition table size in megabytes.
pub const DEFAULT_HASH_MB: usize = 16;

/// Minimum supported transposition table size in megabytes.
pub const MIN_HASH_MB: usize = 1;

/// Maximum supported transposition table size in megabytes.
pub const MAX_HASH_MB: usize = 512;

const BYTES_PER_MIB: usize = 1024 * 1024;
const PROMO_NONE: u16 = 0;
const PROMO_KNIGHT: u16 = 1;
const PROMO_BISHOP: u16 = 2;
const PROMO_ROOK: u16 = 3;
const PROMO_QUEEN: u16 = 4;
const MOVE_VALID_BIT: u16 = 0x8000;
const GENERATION_MASK: u8 = 0x3F;
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
    const fn from_bits(bits: u8) -> Self {
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
    pub(super) depth: u8,
    pub(super) bound: Bound,
}

#[derive(Clone, Copy, Debug)]
struct TtEntry {
    key: u64,
    best_move: u16,
    score: i16,
    depth: u8,
    meta: u8,
}

const _: () = assert!(size_of::<TtEntry>() == 16);

impl TtEntry {
    const EMPTY: Self = Self {
        key: 0,
        best_move: 0,
        score: 0,
        depth: 0,
        meta: 0,
    };

    #[inline]
    const fn pack_meta(bound: Bound, generation: u8) -> u8 {
        ((generation & GENERATION_MASK) << 2) | (bound as u8 & BOUND_MASK)
    }

    #[inline]
    const fn bound(self) -> Bound {
        Bound::from_bits(self.meta)
    }

    #[inline]
    const fn generation(self) -> u8 {
        self.meta >> 2
    }
}

/// Fixed-bucket power-of-two Zobrist transposition table.
#[derive(Debug)]
pub(super) struct TranspositionTable {
    entries: Vec<TtEntry>,
    mask: usize,
    hash_mb: usize,
    generation: u8,
}

impl TranspositionTable {
    pub(super) fn new(hash_mb: usize) -> Self {
        let clamped_mb = clamp_hash_mb(hash_mb);
        let len = entry_count_for_mb(clamped_mb);
        Self {
            entries: vec![TtEntry::EMPTY; len],
            mask: len - 1,
            hash_mb: clamped_mb,
            generation: 0,
        }
    }

    pub(super) const fn hash_mb(&self) -> usize {
        self.hash_mb
    }

    pub(super) fn resize(&mut self, hash_mb: usize) {
        let clamped_mb = clamp_hash_mb(hash_mb);
        if clamped_mb == self.hash_mb && !self.entries.is_empty() {
            return;
        }
        let len = entry_count_for_mb(clamped_mb);
        self.entries = vec![TtEntry::EMPTY; len];
        self.mask = len - 1;
        self.hash_mb = clamped_mb;
        self.generation = 0;
    }

    pub(super) fn clear(&mut self) {
        self.entries.fill(TtEntry::EMPTY);
        self.generation = 0;
    }

    pub(super) const fn new_search(&mut self) {
        self.generation = self.generation.wrapping_add(1) & GENERATION_MASK;
    }

    #[inline]
    pub(super) fn probe(&self, key: u64) -> Option<TtHit> {
        let idx = self.index(key);
        let entry = self.entries[idx];
        let bound = entry.bound();
        if bound == Bound::None || entry.key != key {
            return None;
        }
        Some(TtHit {
            best_move: unpack_move(entry.best_move),
            score: i32::from(entry.score),
            depth: entry.depth,
            bound,
        })
    }

    #[inline]
    pub(super) fn store(
        &mut self,
        key: u64,
        depth: u8,
        bound: Bound,
        score: i32,
        best_move: Option<Move>,
    ) {
        debug_assert_ne!(bound, Bound::None);

        let idx = self.index(key);
        let generation = self.generation;
        let slot = &mut self.entries[idx];
        let existing_bound = slot.bound();
        let same_key = existing_bound != Bound::None && slot.key == key;

        let packed_move = match best_move {
            Some(mv) => pack_move(mv),
            None if same_key => slot.best_move,
            None => 0,
        };

        let should_replace = existing_bound == Bound::None
            || if same_key {
                depth >= slot.depth || (bound == Bound::Exact && existing_bound != Bound::Exact)
            } else {
                slot.generation() != generation || depth >= slot.depth
            };

        if should_replace {
            #[allow(clippy::cast_possible_truncation)]
            let clamped_score = score.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            *slot = TtEntry {
                key,
                best_move: packed_move,
                score: clamped_score,
                depth,
                meta: TtEntry::pack_meta(bound, generation),
            };
        } else if same_key && slot.best_move == 0 && packed_move != 0 {
            slot.best_move = packed_move;
        }
    }

    #[inline]
    const fn index(&self, key: u64) -> usize {
        #[allow(clippy::cast_possible_truncation)]
        {
            (key as usize) & self.mask
        }
    }
}

/// Converts a root-relative search score into a node-relative TT score.
#[inline]
pub(super) const fn score_to_tt(score: i32, ply: usize) -> i32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let ply_i32 = ply as i32;
    if score >= MATE_THRESHOLD {
        score + ply_i32
    } else if score <= -MATE_THRESHOLD {
        score - ply_i32
    } else {
        score
    }
}

/// Converts a node-relative TT score back into a root-relative score at `ply`.
#[inline]
pub(super) const fn score_from_tt(score: i32, ply: usize) -> i32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let ply_i32 = ply as i32;
    if score >= MATE_THRESHOLD {
        score - ply_i32
    } else if score <= -MATE_THRESHOLD {
        score + ply_i32
    } else {
        score
    }
}

#[inline]
const fn clamp_hash_mb(hash_mb: usize) -> usize {
    if hash_mb < MIN_HASH_MB {
        MIN_HASH_MB
    } else if hash_mb > MAX_HASH_MB {
        MAX_HASH_MB
    } else {
        hash_mb
    }
}

#[inline]
const fn entry_count_for_mb(hash_mb: usize) -> usize {
    let bytes = hash_mb.saturating_mul(BYTES_PER_MIB);
    let max_entries = bytes / size_of::<TtEntry>();
    if max_entries <= 1 {
        1
    } else {
        1usize << max_entries.ilog2()
    }
}

#[inline]
const fn pack_move(mv: Move) -> u16 {
    let from = mv.from().as_u8() as u16;
    let to = mv.to().as_u8() as u16;
    let promo = match mv.promotion() {
        Some(PieceType::Knight) => PROMO_KNIGHT,
        Some(PieceType::Bishop) => PROMO_BISHOP,
        Some(PieceType::Rook) => PROMO_ROOK,
        Some(PieceType::Queen) => PROMO_QUEEN,
        None | Some(PieceType::Pawn | PieceType::King) => PROMO_NONE,
    };
    MOVE_VALID_BIT | from | (to << 5) | (promo << 10)
}

#[inline]
const fn unpack_move(raw: u16) -> Option<Move> {
    if (raw & MOVE_VALID_BIT) == 0 {
        return None;
    }
    #[allow(clippy::cast_possible_truncation)]
    let Some(from) = Square::from_index((raw & 0x1F) as u8) else {
        return None;
    };
    #[allow(clippy::cast_possible_truncation)]
    let Some(to) = Square::from_index(((raw >> 5) & 0x1F) as u8) else {
        return None;
    };
    let promo = match (raw >> 10) & 0x07 {
        PROMO_NONE => None,
        PROMO_KNIGHT => Some(PieceType::Knight),
        PROMO_BISHOP => Some(PieceType::Bishop),
        PROMO_ROOK => Some(PieceType::Rook),
        PROMO_QUEEN => Some(PieceType::Queen),
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
        let cases = [
            Move::new(Square::A1, Square::E5, None),
            Move::new(Square::B4, Square::B5, Some(PieceType::Knight)),
            Move::new(Square::C4, Square::D5, Some(PieceType::Bishop)),
            Move::new(Square::D2, Square::D1, Some(PieceType::Rook)),
            Move::new(Square::E4, Square::E5, Some(PieceType::Queen)),
        ];
        for mv in cases {
            assert_eq!(unpack_move(pack_move(mv)), Some(mv));
        }
        assert_eq!(unpack_move(0), None);
    }

    #[test]
    fn mate_score_adjustments_roundtrip_across_plies() {
        for ply in [0, 1, 5, 20, 63] {
            let mate = mate_score_in(ply);
            assert_eq!(score_to_tt(mate, ply), MATE_SCORE);
            assert_eq!(score_from_tt(MATE_SCORE, ply), mate);

            let mated = mated_score_in(ply);
            assert_eq!(score_to_tt(mated, ply), -MATE_SCORE);
            assert_eq!(score_from_tt(-MATE_SCORE, ply), mated);
        }
        assert_eq!(score_to_tt(145, 7), 145);
        assert_eq!(score_from_tt(-230, 12), -230);
    }

    #[test]
    fn store_probe_preserves_move_and_ages_cleanly() {
        let mut tt = TranspositionTable::new(1);
        let key = 0x1234_5678_9ABC_DEF0;
        let mv = Move::new(Square::C2, Square::C3, None);

        tt.store(key, 4, Bound::Exact, 120, Some(mv));
        let hit = tt.probe(key).expect("entry should be present");
        assert_eq!(hit.best_move, Some(mv));
        assert_eq!(hit.score, 120);
        assert_eq!(hit.depth, 4);
        assert_eq!(hit.bound, Bound::Exact);

        // Fail-low at shallower depth must not clobber deeper Exact score or existing best_move.
        tt.store(key, 2, Bound::Upper, 50, None);
        let kept = tt.probe(key).expect("deeper exact entry preserved");
        assert_eq!(kept.best_move, Some(mv));
        assert_eq!(kept.score, 120);
        assert_eq!(kept.depth, 4);

        // Advancing search generation allows replacing with a new position mapping to same bucket.
        tt.new_search();
        let colliding_key = key ^ (1u64 << 40);
        tt.store(colliding_key, 1, Bound::Lower, -40, None);
        assert!(tt.probe(key).is_none());
        assert_eq!(
            tt.probe(colliding_key).map(|h| (h.depth, h.score)),
            Some((1, -40))
        );

        tt.clear();
        assert!(tt.probe(colliding_key).is_none());
    }
}
