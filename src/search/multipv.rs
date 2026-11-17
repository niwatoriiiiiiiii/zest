use crate::board::Board;
use crate::core::Move;
use crate::movegen::{MoveList, generate_legal_moves, in_check};
use crate::search::engine::Searcher;
use crate::search::state::NodeCtx;
use crate::search::types::{PvLine, SearchResult, mated_score_in};
use crate::search::wdl::board_phase;

pub(super) fn terminal_root_result(board: &Board) -> SearchResult {
    let terminal_score = if in_check(board) {
        mated_score_in(0)
    } else {
        0
    };
    SearchResult {
        best_move: None,
        score: terminal_score,
        depth: 0,
        nodes: 1,
        hashfull: 0,
        pv: Vec::new(),
        pvs: Vec::new(),
        phase: board_phase(board),
    }
}

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

    pub(super) fn extend_pv_from_tt(&self, root: &Board, pv: &mut Vec<Move>, max_len: usize) {
        if pv.len() >= max_len {
            return;
        }
        let mut cur = *root;
        let mut seen = Vec::with_capacity(max_len + 1);
        seen.push(cur.key());
        for &mv in pv.iter() {
            cur.make_move(mv);
            seen.push(cur.key());
        }
        while pv.len() < max_len && !cur.is_insufficient_material() {
            let Some(mv) = self.tt.probe(cur.key()).and_then(|hit| hit.best_move) else {
                break;
            };
            if !generate_legal_moves(&cur).as_slice().contains(&mv) {
                break;
            }
            cur.make_move(mv);
            let next_key = cur.key();
            if seen.contains(&next_key) {
                break;
            }
            seen.push(next_key);
            pv.push(mv);
        }
    }
}
