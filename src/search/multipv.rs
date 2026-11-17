use crate::board::Board;
use crate::core::Move;
use crate::movegen::MoveList;
use crate::search::engine::Searcher;
use crate::search::state::NodeCtx;
use crate::search::types::PvLine;

pub(super) struct MultiPvState {
    pub(super) scores: Vec<i32>,
    pub(super) hints: Vec<Option<Move>>,
}

impl MultiPvState {
    pub(super) fn new(count: usize, init_score: i32) -> Self {
        Self {
            scores: vec![init_score; count],
            hints: vec![None; count],
        }
    }
}

impl Searcher {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn search_multipv_iter(
        &mut self,
        board: &Board,
        root_moves: &MoveList,
        depth: usize,
        depth_u8: u8,
        state: &mut MultiPvState,
        root_ctx: NodeCtx<'_>,
    ) -> Option<Vec<PvLine>> {
        let target_pvs = state.scores.len();
        let mut remaining = *root_moves;
        let mut lines = Vec::with_capacity(target_pvs);

        for idx in 0..target_pvs {
            if remaining.is_empty() {
                break;
            }
            let score = self.search_aspiration(
                board,
                &remaining,
                depth,
                depth_u8,
                state.scores[idx],
                &mut state.hints[idx],
                root_ctx,
            );
            if self.state.aborted {
                return None;
            }
            let root_len = self.state.pv_len[0];
            let (candidate, mut pv) = if root_len > 0 {
                (
                    self.state.pv_table[0][0],
                    self.state.pv_table[0][..root_len].to_vec(),
                )
            } else {
                let mv = remaining[0];
                (mv, vec![mv])
            };
            self.extend_pv_from_tt(board, &mut pv, depth);
            lines.push(PvLine { score, moves: pv });
            remaining.retain(|&mv| mv != candidate);
        }

        lines.sort_by_key(|line| core::cmp::Reverse(line.score));
        for (idx, line) in lines.iter().enumerate() {
            state.scores[idx] = line.score;
            state.hints[idx] = line.moves.first().copied();
        }
        Some(lines)
    }
}
