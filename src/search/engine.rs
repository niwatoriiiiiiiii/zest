use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::core::Move;
use crate::eval::evaluate;
use crate::movegen::{MoveList, generate_legal_moves, in_check};
use crate::search::ordering::{order_moves, pick_next_move};
use crate::search::pruning::{MovePruneCtx, PreparedNode, lmr_reduction};
use crate::search::quiescence::quiesce;
use crate::search::state::{NodeCtx, SearchState, is_quiet_move};
use crate::search::tt::{DEFAULT_HASH_MB, TranspositionTable};
use crate::search::tt_entry::Bound;
use crate::search::types::{INF_SCORE, MAX_PLY, SearchLimits, SearchResult, mated_score_in};

/// Stateful game-tree search engine using iterative deepening, PVS, and pruning heuristics.
#[derive(Debug)]
pub struct Searcher {
    pub(super) state: SearchState,
    pub(super) tt: TranspositionTable,
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher {
    /// Creates a reusable [`Searcher`] with pre-allocated working buffers and default TT size.
    #[must_use]
    pub fn new() -> Self {
        Self::with_hash_mb(DEFAULT_HASH_MB)
    }

    /// Creates a reusable [`Searcher`] with a transposition table sized to `hash_mb` MiB.
    #[must_use]
    pub fn with_hash_mb(hash_mb: usize) -> Self {
        Self {
            state: SearchState::new(),
            tt: TranspositionTable::new(hash_mb),
        }
    }

    /// Returns the configured transposition table size in MiB.
    #[must_use]
    pub const fn hash_mb(&self) -> usize {
        self.tt.hash_mb()
    }

    /// Resizes the transposition table to `hash_mb` MiB (clamped to supported bounds).
    pub fn set_hash_mb(&mut self, hash_mb: usize) {
        self.tt.resize(hash_mb);
    }

    /// Clears all entries from the transposition table and search ordering heuristics.
    pub fn clear_tt(&mut self) {
        self.tt.clear();
        self.state.clear_heuristics();
    }

    /// Searches `board` using `limits` with optional repetition `history` keys and `halfmove_clock`.
    #[must_use]
    pub fn search(
        &mut self,
        board: &Board,
        history: &[u64],
        halfmove_clock: u16,
        limits: SearchLimits,
    ) -> SearchResult {
        self.search_inner(board, history, halfmove_clock, limits, None)
    }

    /// Searches `board` like [`Self::search`], aborting early if `stop` becomes `true`.
    #[must_use]
    pub fn search_with_stop(
        &mut self,
        board: &Board,
        history: &[u64],
        halfmove_clock: u16,
        limits: SearchLimits,
        stop: &AtomicBool,
    ) -> SearchResult {
        self.search_inner(board, history, halfmove_clock, limits, Some(stop))
    }

    fn negamax(
        &mut self,
        board: &Board,
        depth: u8,
        alpha: i32,
        beta: i32,
        ctx: NodeCtx<'_>,
    ) -> i32 {
        self.state.pv_len[ctx.ply] = ctx.ply;

        if depth == 0 {
            return quiesce(&mut self.state, &mut self.tt, board, alpha, beta, ctx);
        }
        if self.state.check_limits(ctx.stop) {
            return 0;
        }

        self.state.nodes += 1;
        if ctx.ply >= MAX_PLY {
            return evaluate(board);
        }

        let key = board.key();
        if self.state.is_repetition(key, ctx.reversible_start) || board.is_insufficient_material() {
            return 0;
        }

        let is_pv = beta - alpha > 1;
        let tt_probe = self
            .tt
            .probe_cutoff(key, depth, ctx.ply, alpha, beta, ctx.halfmove);
        if !is_pv && let Some((tt_score, _)) = tt_probe.cutoff {
            return tt_score;
        }

        let checked = in_check(board);
        let moves = generate_legal_moves(board);
        if moves.is_empty() {
            return if checked { mated_score_in(ctx.ply) } else { 0 };
        }
        if ctx.halfmove >= 100 {
            return 0;
        }

        let prep = MovePruneCtx::prepare(
            board,
            depth,
            (alpha, beta),
            is_pv,
            checked,
            tt_probe.best_move.is_some(),
            ctx.ply,
        );
        if let Some(score) = prep.rfp_cutoff {
            return score;
        }

        self.search_children(board, &moves, &prep, tt_probe.best_move, (alpha, beta), ctx)
    }

    fn search_children(
        &mut self,
        board: &Board,
        moves: &MoveList,
        prep: &PreparedNode,
        tt_move: Option<Move>,
        (mut alpha, beta): (i32, i32),
        ctx: NodeCtx<'_>,
    ) -> i32 {
        let key = board.key();
        let orig_alpha = alpha;
        let mut best_score = -INF_SCORE;
        let mut best_move = None;
        let mut quiet_count = 0usize;
        let mut searched_quiets = MoveList::new();
        let (mut scored, len) = order_moves(board, moves, tt_move, &self.state, ctx.ply);

        for idx in 0..len {
            let mv = pick_next_move(board, &mut scored, idx, len);
            let quiet = is_quiet_move(board, mv);
            if quiet {
                quiet_count += 1;
            }

            let (prunable, lmr_candidate) =
                prep.prune_ctx
                    .classify(board, mv, idx, quiet, quiet_count, scored[idx].score());
            let mut next_board = *board;
            let child_ctx = ctx.advance(&mut next_board, mv, self.state.path_keys.len());
            let gives_check = (prunable || lmr_candidate) && in_check(&next_board);
            if prunable && !gives_check {
                continue;
            }

            let prior_quiets = searched_quiets.len();
            if quiet {
                searched_quiets.push(mv);
            }

            let red = if lmr_candidate && !gives_check {
                lmr_reduction(
                    prep.depth,
                    prep.child_depth,
                    idx,
                    prep.prune_ctx.is_pv,
                    scored[idx].score(),
                )
            } else {
                0
            };
            let score = self.search_window(
                &next_board,
                prep.child_depth,
                red,
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
                        self.record_beta_cutoff(
                            board,
                            key,
                            prep.depth,
                            best_score,
                            &searched_quiets.as_slice()[..prior_quiets],
                            mv,
                            ctx.ply,
                        );
                        return best_score;
                    }
                    alpha = score;
                    self.state.update_pv(ctx.ply, mv);
                }
            }
        }

        self.store_node_result(key, prep.depth, best_score, orig_alpha, best_move, ctx.ply)
    }

    #[inline]
    pub(super) fn store_node_result(
        &mut self,
        key: u64,
        depth: u8,
        best_score: i32,
        orig_alpha: i32,
        best_move: Option<Move>,
        ply: usize,
    ) -> i32 {
        let (bound, stored_move) = if best_score > orig_alpha {
            (Bound::Exact, best_move)
        } else {
            (Bound::Upper, None)
        };
        self.tt
            .store_at(key, depth, bound, best_score, ply, stored_move);
        best_score
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn record_beta_cutoff(
        &mut self,
        board: &Board,
        key: u64,
        depth: u8,
        best_score: i32,
        tried_quiets: &[Move],
        cutoff_move: Move,
        ply: usize,
    ) {
        self.record_quiet_cutoff(board, tried_quiets, cutoff_move, depth, ply);
        self.tt
            .store_at(key, depth, Bound::Lower, best_score, ply, Some(cutoff_move));
    }

    #[inline]
    fn record_quiet_cutoff(
        &mut self,
        board: &Board,
        tried_quiets: &[Move],
        cutoff_move: Move,
        depth: u8,
        ply: usize,
    ) {
        if !is_quiet_move(board, cutoff_move) {
            return;
        }
        let side = board.side_to_move();
        let bonus = i32::from(depth) * i32::from(depth);
        self.state.record_killer(ply, cutoff_move);
        self.state.update_history(side, cutoff_move, bonus);
        for &prev in tried_quiets {
            self.state.update_history(side, prev, -bonus);
        }
    }

    #[inline]
    pub(super) fn search_window(
        &mut self,
        next_board: &Board,
        child_depth: u8,
        reduction: u8,
        is_first: bool,
        (alpha, beta): (i32, i32),
        child_ctx: NodeCtx<'_>,
    ) -> i32 {
        self.state.path_keys.push(next_board.key());

        let raw = if is_first {
            -self.negamax(next_board, child_depth, -beta, -alpha, child_ctx)
        } else {
            let reduced_depth = child_depth.saturating_sub(reduction);
            let mut score = -self.negamax(next_board, reduced_depth, -alpha - 1, -alpha, child_ctx);
            if reduction > 0 && !self.state.aborted && score > alpha {
                score = -self.negamax(next_board, child_depth, -alpha - 1, -alpha, child_ctx);
            }
            if !self.state.aborted && score > alpha && score < beta {
                score = -self.negamax(next_board, child_depth, -beta, -alpha, child_ctx);
            }
            score
        };

        let result = if self.state.aborted { 0 } else { raw };
        self.state.path_keys.pop();
        result
    }
}

/// Convenience wrapper creating a temporary [`Searcher`] to search `board` once.
#[must_use]
pub fn search(
    board: &Board,
    history: &[u64],
    halfmove_clock: u16,
    limits: SearchLimits,
) -> SearchResult {
    let mut searcher = Searcher::new();
    searcher.search(board, history, halfmove_clock, limits)
}
