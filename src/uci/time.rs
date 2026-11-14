use crate::core::Color;
use crate::search::SearchLimits;
use crate::uci::command::GoParams;

const DEFAULT_SYNC_DEPTH: u8 = 4;
const DEFAULT_MOVES_TO_GO: u16 = 20;
const TIME_RESERVE_MS: u64 = 30;

/// Converts [`GoParams`] into deterministic [`SearchLimits`] for the search engine.
#[must_use]
pub const fn compute_search_limits(params: &GoParams) -> SearchLimits {
    let has_budget = params.infinite
        || params.movetime.is_some()
        || params.wtime.is_some()
        || params.btime.is_some();

    if params.depth.is_none() && params.nodes.is_none() && !has_budget {
        SearchLimits::depth(DEFAULT_SYNC_DEPTH)
    } else {
        SearchLimits {
            max_depth: params.depth,
            max_nodes: params.nodes,
        }
    }
}

/// Computes the target search duration in milliseconds for `side_to_move`, or `None` if unlimited.
#[must_use]
pub fn compute_time_budget_ms(params: &GoParams, side_to_move: Color) -> Option<u64> {
    if params.infinite {
        return None;
    }

    if let Some(movetime) = params.movetime {
        return Some(movetime.max(1));
    }

    let (remaining, increment) = match side_to_move {
        Color::White => (params.wtime, params.winc.unwrap_or(0)),
        Color::Black => (params.btime, params.binc.unwrap_or(0)),
    };

    let remaining_ms = remaining?;
    let usable_ms = remaining_ms.saturating_sub(TIME_RESERVE_MS).max(1);
    let moves_to_go = u64::from(params.movestogo.unwrap_or(DEFAULT_MOVES_TO_GO).max(1));

    let base = usable_ms / moves_to_go + (increment.saturating_mul(3)) / 4;
    let max_budget = if params.movestogo == Some(1) {
        usable_ms
    } else {
        (usable_ms / 2).max(1)
    };

    Some(base.clamp(1, max_budget))
}
