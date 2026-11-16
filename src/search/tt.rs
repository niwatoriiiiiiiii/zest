use crate::core::Move;
use crate::search::tt_entry::{
    Bound, EVAL_NONE, GENERATION_MASK, TtEntry, adjust_mate_score, pack_move, unpack_move,
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
        let len = if max_entries <= 2 {
            2
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
        let base = self.bucket_base(key);
        for slot in &self.entries[base..base + 2] {
            let bound = slot.bound();
            if bound != Bound::None && slot.key == key {
                return Some(TtHit {
                    best_move: unpack_move(slot.best_move),
                    score: i32::from(slot.score),
                    eval: (slot.eval != EVAL_NONE).then(|| i32::from(slot.eval)),
                    depth: slot.depth,
                    bound,
                });
            }
        }
        None
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
                static_eval: None,
            };
        };
        let best_move = hit.best_move;
        let static_eval = hit.eval;
        if halfmove_clock >= 100 || hit.depth < depth {
            return TtProbe {
                cutoff: None,
                best_move,
                static_eval,
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
            static_eval,
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn store_at(
        &mut self,
        key: u64,
        depth: u8,
        bound: Bound,
        score: i32,
        ply: usize,
        best_move: Option<Move>,
        static_eval: Option<i32>,
    ) {
        self.store(
            key,
            depth,
            bound,
            adjust_mate_score(score, ply, true),
            best_move,
            static_eval,
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
        static_eval: Option<i32>,
    ) {
        debug_assert_ne!(bound, Bound::None);

        let base = self.bucket_base(key);
        let generation = self.generation;
        let idx = self.select_bucket_slot(base, key, generation);
        let slot = &mut self.entries[idx];
        let existing_bound = slot.bound();
        let same_key = existing_bound != Bound::None && slot.key == key;

        let packed_move = match best_move {
            Some(mv) => pack_move(mv),
            None if same_key => slot.best_move,
            None => 0,
        };
        #[allow(clippy::cast_possible_truncation)]
        let packed_eval = static_eval.map_or(EVAL_NONE, |eval| {
            eval.clamp(i32::from(EVAL_NONE) + 1, i32::from(i16::MAX)) as i16
        });

        let should_replace = existing_bound == Bound::None
            || if same_key {
                depth >= slot.depth || (bound == Bound::Exact && existing_bound != Bound::Exact)
            } else {
                slot.generation() != generation || depth >= slot.depth
            };

        if should_replace {
            let final_eval = if packed_eval != EVAL_NONE {
                packed_eval
            } else if same_key {
                slot.eval
            } else {
                EVAL_NONE
            };
            #[allow(clippy::cast_possible_truncation)]
            let clamped_score = score.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
            *slot = TtEntry {
                key,
                best_move: packed_move,
                score: clamped_score,
                eval: final_eval,
                depth,
                meta: TtEntry::pack_meta(bound, generation),
            };
        } else if same_key {
            if slot.best_move == 0 && packed_move != 0 {
                slot.best_move = packed_move;
            }
            if slot.eval == EVAL_NONE && packed_eval != EVAL_NONE {
                slot.eval = packed_eval;
            }
        }
    }

    #[inline]
    fn select_bucket_slot(&self, base: usize, key: u64, generation: u8) -> usize {
        let s0 = self.entries[base];
        let b0 = s0.bound();
        if b0 != Bound::None && s0.key == key {
            return base;
        }
        let s1 = self.entries[base + 1];
        let b1 = s1.bound();
        if b1 != Bound::None && s1.key == key {
            return base + 1;
        }
        if b0 == Bound::None {
            return base;
        }
        if b1 == Bound::None {
            return base + 1;
        }
        let age0 = u16::from(s0.generation() != generation) * 256 + u16::from(u8::MAX - s0.depth);
        let age1 = u16::from(s1.generation() != generation) * 256 + u16::from(u8::MAX - s1.depth);
        if age0 >= age1 { base } else { base + 1 }
    }

    #[inline]
    const fn bucket_base(&self, key: u64) -> usize {
        #[allow(clippy::cast_possible_truncation)]
        {
            (key as usize) & (self.mask & !1)
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

        tt.store(key, 4, Bound::Exact, 120, Some(mv), Some(42));
        let hit = tt.probe(key).expect("entry should be present");
        assert_eq!(
            (hit.best_move, hit.score, hit.eval, hit.depth, hit.bound),
            (Some(mv), 120, Some(42), 4, Bound::Exact)
        );

        tt.store(key, 2, Bound::Upper, 50, None, None);
        let kept = tt.probe(key).expect("deeper exact entry preserved");
        assert_eq!(
            (kept.best_move, kept.score, kept.eval, kept.depth),
            (Some(mv), 120, Some(42), 4)
        );

        tt.new_search();
        let colliding_key = key ^ (1u64 << 40);
        tt.store(colliding_key, 1, Bound::Lower, -40, None, None);
        assert!(tt.probe(key).is_some(), "second bucket slot preserves key");
        assert_eq!(
            tt.probe(colliding_key).map(|h| (h.depth, h.score)),
            Some((1, -40))
        );

        let colliding_key2 = key ^ (2u64 << 40);
        tt.store(colliding_key2, 2, Bound::Lower, 20, None, None);
        assert!(tt.probe(key).is_none(), "older generation entry aged out");
        assert!(tt.probe(colliding_key2).is_some());

        tt.clear();
        assert!(tt.probe(colliding_key2).is_none());
        assert_eq!(tt.hashfull_permille(), 0);
    }

    #[test]
    fn hashfull_permille_counts_current_generation_entries() {
        let mut tt = TranspositionTable::new(1);
        assert_eq!(tt.hashfull_permille(), 0);
        for slot_idx in 0u64..100 {
            tt.store(slot_idx, 1, Bound::Exact, 0, None, None);
        }
        assert_eq!(tt.hashfull_permille(), 100);
        tt.new_search();
        assert_eq!(tt.hashfull_permille(), 0);
    }
}
