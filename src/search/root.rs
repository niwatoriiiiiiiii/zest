use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::core::Move;
use crate::eval::evaluate_with_acc;
use crate::movegen::{MoveList, generate_legal_moves};
use crate::search::engine::Searcher;
use crate::search::multipv::{MultiPvState, terminal_root_result};
use crate::search::ordering::{order_moves, pick_next_move};
use crate::search::state::NodeCtx;
use crate::search::tt_entry::Bound;
use crate::search::types::{
    DEFAULT_MULTI_PV, INF_SCORE, MAX_MULTI_PV, MAX_PLY, MIN_MULTI_PV, PvLine, SearchLimits,
    SearchResult, is_mate_score,
};

const ASPIRATION_MIN_DEPTH: usize = 4;
const ASPIRATION_INITIAL_DELTA: i32 = 50;
const ASPIRATION_MAX_DELTA: i32 = 400;

impl Searcher {
    pub(super) fn search_inner(
        &mut self,
        board: &Board,
        history: &[u64],
        halfmove_clock: u16,
        limits: SearchLimits,
        stop: Option<&AtomicBool>,
    ) -> SearchResult {
        let root_moves = generate_legal_moves(board);
        if root_moves.is_empty() {
            return terminal_root_result(board);
        }

        self.state.reset(board, history, limits.max_nodes);
        self.tt.new_search();

        let fallback_move = root_moves[0];
        let mut best_move = fallback_move;
        let mut best_score = evaluate_with_acc(board, &mut self.state.nnue_acc, 0);
        let mut completed_depth = 0u8;
        let mut best_pv = vec![fallback_move];
        let mut pv_hint = None;

        let target_pvs = limits
            .multi_pv
            .unwrap_or(DEFAULT_MULTI_PV)
            .clamp(MIN_MULTI_PV, MAX_MULTI_PV)
            .min(root_moves.len());
        let mut best_pvs: Vec<PvLine> = Vec::new();
        let mut multi_state = (target_pvs > 1).then(|| MultiPvState::new(target_pvs, best_score));

        let max_depth = usize::from(limits.max_depth.unwrap_or(64)).clamp(1, MAX_PLY);
        let root_reversible_start = self
            .state
            .path_keys
            .len()
            .saturating_sub(1 + usize::from(halfmove_clock));
        let root_ctx = NodeCtx {
            ply: 0,
            halfmove: halfmove_clock,
            reversible_start: root_reversible_start,
            allow_null: true,
            stop,
        };

        for depth in 1..=max_depth {
            if self.state.should_stop(stop) {
                break;
            }

            #[allow(clippy::cast_possible_truncation)]
            let depth_u8 = depth as u8;
            if target_pvs == 1 {
                let score = self.search_aspiration(
                    board,
                    &root_moves,
                    depth,
                    depth_u8,
                    best_score,
                    &mut pv_hint,
                    root_ctx,
                );
                if self.state.aborted {
                    break;
                }
                let root_len = self.state.pv_len[0];
                if root_len > 0 {
                    let candidate = self.state.pv_table[0][0];
                    best_move = candidate;
                    best_score = score;
                    completed_depth = depth_u8;
                    best_pv = self.state.pv_table[0][..root_len].to_vec();
                    self.extend_pv_from_tt(board, &mut best_pv, depth);
                    pv_hint = Some(candidate);
                }
            } else if let Some(ref mut ms) = multi_state {
                let Some(iter_pvs) =
                    self.search_multipv_iter(board, &root_moves, depth, depth_u8, ms, root_ctx)
                else {
                    break;
                };
                if let Some(top) = iter_pvs.first()
                    && let Some(&mv) = top.moves.first()
                {
                    best_move = mv;
                    best_score = top.score;
                    completed_depth = depth_u8;
                    best_pv.clone_from(&top.moves);
                    best_pvs = iter_pvs;
                }
            }
        }

        if best_pvs.is_empty() {
            best_pvs.push(PvLine {
                score: best_score,
                moves: best_pv.clone(),
            });
        }

        SearchResult {
            best_move: Some(best_move),
            score: best_score,
            depth: completed_depth,
            nodes: self.state.nodes,
            hashfull: self.tt.hashfull_permille(),
            pv: best_pv,
            pvs: best_pvs,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn search_aspiration(
        &mut self,
        board: &Board,
        root_moves: &MoveList,
        depth: usize,
        depth_u8: u8,
        prev_score: i32,
        pv_hint: &mut Option<Move>,
        root_ctx: NodeCtx<'_>,
    ) -> i32 {
        let mut delta = ASPIRATION_INITIAL_DELTA;
        let (mut alpha, mut beta) = if depth >= ASPIRATION_MIN_DEPTH && !is_mate_score(prev_score) {
            (
                (prev_score - delta).max(-INF_SCORE),
                (prev_score + delta).min(INF_SCORE),
            )
        } else {
            (-INF_SCORE, INF_SCORE)
        };

        loop {
            let score =
                self.negamax_root(board, root_moves, depth_u8, alpha, beta, *pv_hint, root_ctx);
            if self.state.aborted {
                return 0;
            }
            if score <= alpha && alpha > -INF_SCORE {
                alpha = if is_mate_score(score) || delta >= ASPIRATION_MAX_DELTA {
                    -INF_SCORE
                } else {
                    (alpha - delta).max(-INF_SCORE)
                };
                delta = delta.saturating_mul(2);
            } else if score >= beta && beta < INF_SCORE {
                if self.state.pv_len[0] > 0 {
                    *pv_hint = Some(self.state.pv_table[0][0]);
                }
                beta = if is_mate_score(score) || delta >= ASPIRATION_MAX_DELTA {
                    INF_SCORE
                } else {
                    (beta + delta).min(INF_SCORE)
                };
                delta = delta.saturating_mul(2);
            } else {
                return score;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn negamax_root(
        &mut self,
        board: &Board,
        moves: &MoveList,
        depth: u8,
        mut alpha: i32,
        beta: i32,
        pv_hint: Option<Move>,
        ctx: NodeCtx<'_>,
    ) -> i32 {
        self.state.nodes += 1;
        self.state.pv_len[0] = 0;

        let key = board.key();
        let tt_hit = self.tt.probe(key);
        let priority_move = match tt_hit {
            Some(hit) if hit.depth >= depth && hit.best_move.is_some() => hit.best_move,
            _ => pv_hint.or_else(|| tt_hit.and_then(|hit| hit.best_move)),
        };

        let orig_alpha = alpha;
        let mut best_score = -INF_SCORE;
        let mut best_move = None;
        let (mut scored, len) = order_moves(board, moves, priority_move, &self.state, 0);

        for idx in 0..len {
            let mv = pick_next_move(board, &mut scored, idx, len).mv();
            self.state.nnue_acc.push_move(ctx.ply + 1, board, mv);
            let mut next_board = *board;
            let child_ctx = ctx.advance(&mut next_board, mv, self.state.path_keys.len());
            let score = self.search_window(
                &next_board,
                depth - 1,
                0,
                idx == 0,
                (alpha, beta),
                child_ctx,
            );
            if self.state.aborted {
                return 0;
            }
            if score > best_score {
                best_score = score;
                best_move = Some(mv);
                if score > alpha {
                    if score >= beta {
                        self.state.update_pv(0, mv);
                        self.tt.store_at(
                            key,
                            depth,
                            Bound::Lower,
                            best_score,
                            0,
                            best_move,
                            tt_hit.and_then(|hit| hit.eval),
                        );
                        return best_score;
                    }
                    alpha = score;
                    self.state.update_pv(0, mv);
                }
            }
        }

        self.store_node_result(
            key,
            depth,
            best_score,
            orig_alpha,
            best_move,
            tt_hit.and_then(|hit| hit.eval),
            0,
        )
    }
}
