use crate::core::Move;
use crate::search::tt_entry::{
    Bound, GENERATION_MASK, TtEntry, adjust_mate_score, pack_move, unpack_move,
};

/// Default transposition table size in megabytes.
pub const DEFAULT_HASH_MB: usize = 16;

/// Minimum supported transposition table size in megabytes.
pub const MIN_HASH_MB: usize = 1;

/// Maximum supported transposition table size in megabytes.
pub const MAX_HASH_MB: usize = 512;

const BYTES_PER_MIB: usize = 1024 * 1024;

/// Decoded transposition table lookup result.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct TtHit {
    pub(super) best_move: Option<Move>,
    pub(super) score: i32,
    pub(super) depth: u8,
    pub(super) bound: Bound,
}

/// Outcome of probing the TT against a search window `(alpha, beta)`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct TtProbe {
    pub(super) cutoff: Option<(i32, bool)>,
    pub(super) best_move: Option<Move>,
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
        let (entries, mask, clamped_mb) = Self::alloc_table(hash_mb);
        Self {
            entries,
            mask,
            hash_mb: clamped_mb,
            generation: 0,
        }
    }

    fn alloc_table(hash_mb: usize) -> (Vec<TtEntry>, usize, usize) {
        let clamped_mb = hash_mb.clamp(MIN_HASH_MB, MAX_HASH_MB);
        let max_entries = clamped_mb.saturating_mul(BYTES_PER_MIB) / size_of::<TtEntry>();
        let len = if max_entries <= 1 {
            1
        } else {
            1usize << max_entries.ilog2()
        };
        (vec![TtEntry::EMPTY; len], len - 1, clamped_mb)
    }

    pub(super) const fn hash_mb(&self) -> usize {
        self.hash_mb
    }

    pub(super) fn resize(&mut self, hash_mb: usize) {
        let clamped_mb = hash_mb.clamp(MIN_HASH_MB, MAX_HASH_MB);
        if clamped_mb == self.hash_mb && !self.entries.is_empty() {
            return;
        }
        let (entries, mask, clamped_mb) = Self::alloc_table(clamped_mb);
        self.entries = entries;
        self.mask = mask;
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

    /// Estimates table occupancy in permille (`0..=1000`) by sampling the leading entries
    /// written in the current search generation, matching the UCI `hashfull` convention.
    #[must_use]
    pub(super) fn hashfull_permille(&self) -> u16 {
        let sample_len = self.entries.len().min(1000);
        if sample_len == 0 {
            return 0;
        }
        let used = self.entries[..sample_len]
            .iter()
            .filter(|entry| entry.bound() != Bound::None && entry.generation() == self.generation)
            .count();
        #[allow(clippy::cast_possible_truncation)]
        {
            ((used * 1000) / sample_len) as u16
        }
    }

    #[inline]
    pub(super) fn probe(&self, key: u64) -> Option<TtHit> {
        let entry = self.entries[self.index(key)];
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
    pub(super) fn probe_cutoff(
        &self,
        key: u64,
        depth: u8,
        ply: usize,
        alpha: i32,
        beta: i32,
        halfmove_clock: u16,
    ) -> TtProbe {
        let Some(hit) = self.probe(key) else {
            return TtProbe {
                cutoff: None,
                best_move: None,
            };
        };
        let best_move = hit.best_move;
        if halfmove_clock >= 100 || hit.depth < depth {
            return TtProbe {
                cutoff: None,
                best_move,
            };
        }

        let tt_score = adjust_mate_score(hit.score, ply, false);
        let can_cut = match hit.bound {
            Bound::Exact => true,
            Bound::Lower => tt_score >= beta,
            Bound::Upper => tt_score <= alpha,
            Bound::None => false,
        };
        TtProbe {
            cutoff: can_cut.then_some((tt_score, hit.bound == Bound::Exact)),
            best_move,
        }
    }

    #[inline]
    pub(super) fn store_at(
        &mut self,
        key: u64,
        depth: u8,
        bound: Bound,
        score: i32,
        ply: usize,
        best_move: Option<Move>,
    ) {
        self.store(
            key,
            depth,
            bound,
            adjust_mate_score(score, ply, true),
            best_move,
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Square;

    #[test]
    fn store_probe_preserves_move_and_ages_cleanly() {
        let mut tt = TranspositionTable::new(1);
        let key = 0x1234_5678_9ABC_DEF0;
        let mv = Move::new(Square::C2, Square::C3, None);

        tt.store(key, 4, Bound::Exact, 120, Some(mv));
        let hit = tt.probe(key).expect("entry should be present");
        assert_eq!(
            (hit.best_move, hit.score, hit.depth, hit.bound),
            (Some(mv), 120, 4, Bound::Exact)
        );

        tt.store(key, 2, Bound::Upper, 50, None);
        let kept = tt.probe(key).expect("deeper exact entry preserved");
        assert_eq!((kept.best_move, kept.score, kept.depth), (Some(mv), 120, 4));

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
        assert_eq!(tt.hashfull_permille(), 0);
    }

    #[test]
    fn hashfull_permille_counts_current_generation_entries() {
        let mut tt = TranspositionTable::new(1);
        assert_eq!(tt.hashfull_permille(), 0);
        for slot_idx in 0u64..100 {
            tt.store(slot_idx, 1, Bound::Exact, 0, None);
        }
        assert_eq!(tt.hashfull_permille(), 100);
        tt.new_search();
        assert_eq!(tt.hashfull_permille(), 0);
    }
}
