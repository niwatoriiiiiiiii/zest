use crate::search::SearchLimits;
use crate::uci::command::GoParams;

const DEFAULT_SYNC_DEPTH: u8 = 4;

/// Converts [`GoParams`] into deterministic [`SearchLimits`] for the search engine.
#[must_use]
pub const fn compute_search_limits(params: &GoParams) -> SearchLimits {
    if params.depth.is_none() && params.nodes.is_none() {
        SearchLimits::depth(DEFAULT_SYNC_DEPTH)
    } else {
        SearchLimits {
            max_depth: params.depth,
            max_nodes: params.nodes,
        }
    }
}
