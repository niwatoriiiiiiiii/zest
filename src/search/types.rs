use crate::core::Move;

/// Checkmates score magnitude used at ply 0.
///
/// Leaves headroom well below `i32::MAX` so negation never overflows and mate distance
/// (`MATE_SCORE - ply`) remains strictly distinguishable from static evaluations.
pub const MATE_SCORE: i32 = 30_000;

/// Strict upper bound (`-INF_SCORE..=INF_SCORE`) used for initial alpha-beta windows.
pub const INF_SCORE: i32 = 31_000;

/// Threshold above which a search score represents a forced mate within `MAX_PLY` plies.
pub const MATE_THRESHOLD: i32 = MATE_SCORE - 256;

/// Maximum supported search ply depth.
pub const MAX_PLY: usize = 64;

/// Returns the mate score for a side delivering checkmate at `ply` half-moves from the root.
#[must_use]
#[inline]
pub const fn mate_score_in(ply: usize) -> i32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    {
        MATE_SCORE - (ply as i32)
    }
}

/// Returns the mated score for a side being checkmated at `ply` half-moves from the root.
#[must_use]
#[inline]
pub const fn mated_score_in(ply: usize) -> i32 {
    -mate_score_in(ply)
}

/// Returns `true` if `score` encodes a forced mate or mated distance.
#[must_use]
#[inline]
pub const fn is_mate_score(score: i32) -> bool {
    score.abs() >= MATE_THRESHOLD
}

/// Converts a mate score into full moves until checkmate (positive = winning, negative = losing).
#[must_use]
#[inline]
pub const fn mate_distance_in_moves(score: i32) -> Option<i32> {
    if !is_mate_score(score) {
        return None;
    }
    let plies = MATE_SCORE - score.abs();
    let moves = (plies + 1) / 2;
    Some(if score > 0 { moves } else { -moves })
}

/// Resource limits controlling when a search stops.
///
/// Designed without `std::time` dependencies so engine search remains identical and deterministic
/// on WASM targets as well as native hosts.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct SearchLimits {
    /// Maximum iterative-deepening depth to search. Defaults to `MAX_PLY` if omitted.
    pub max_depth: Option<u8>,

    /// Maximum number of visited nodes (main search + quiescence) before aborting.
    pub max_nodes: Option<u64>,
}

impl SearchLimits {
    /// Creates limits bounded only by `max_depth`.
    #[must_use]
    pub const fn depth(max_depth: u8) -> Self {
        Self {
            max_depth: Some(max_depth),
            max_nodes: None,
        }
    }

    /// Creates limits bounded only by `max_nodes`.
    #[must_use]
    pub const fn nodes(max_nodes: u64) -> Self {
        Self {
            max_depth: None,
            max_nodes: Some(max_nodes),
        }
    }

    /// Creates limits bounded by both `max_depth` and `max_nodes`.
    #[must_use]
    pub const fn depth_and_nodes(max_depth: u8, max_nodes: u64) -> Self {
        Self {
            max_depth: Some(max_depth),
            max_nodes: Some(max_nodes),
        }
    }
}

/// Result produced by a completed or interrupted search.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SearchResult {
    /// Best move found from the root position, or `None` if no legal moves exist.
    pub best_move: Option<Move>,

    /// Score in centipawns from the root side-to-move perspective (or mate score).
    pub score: i32,

    /// Deepest search iteration completed cleanly (`0` if terminal or interrupted at depth 1).
    pub depth: u8,

    /// Total nodes visited across all iterations and quiescence calls.
    pub nodes: u64,

    /// Principal variation starting from the root position.
    pub pv: Vec<Move>,
}
