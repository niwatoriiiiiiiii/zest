use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::core::{Color, Move, PieceType};
use crate::eval::evaluate_with_acc;
use crate::movegen::{MoveList, generate_legal_moves, gives_check, in_check, is_legal_move};
use crate::search::ordering::{order_moves, pick_next_move};
use crate::search::pruning::{MovePruneCtx, PreparedNode, lmr_reduction};
use crate::search::quiescence::quiesce;
use crate::search::state::{NodeCtx, SearchState, is_quiet_move};
use crate::search::tt::{DEFAULT_HASH_MB, TranspositionTable};
use crate::search::tt_entry::Bound;
use crate::search::types::{
    INF_SCORE, MAX_PLY, SearchLimits, SearchResult, is_mate_score, mated_score_in,
};

struct NodeSearch {
    orig_alpha: i32,
    alpha: i32,
    best_score: i32,
    best_move: Option<Move>,
    quiet_count: usize,
    searched_quiets: MoveList,
    skip_move: Option<Move>,
}

impl NodeSearch {
    #[inline]
    const fn new(alpha: i32) -> Self {
        Self {
            orig_alpha: alpha,
            alpha,
            best_score: -INF_SCORE,
            best_move: None,
            quiet_count: 0,
            searched_quiets: MoveList::new(),
            skip_move: None,
        }
    }
}

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
            return evaluate_with_acc(board, &mut self.state.nnue_acc, ctx.ply);
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
        if !checked && ctx.halfmove >= 100 {
            return 0;
        }

        let mut static_eval = (!checked).then_some(tt_probe.static_eval).flatten();
        let need_eval = !checked
            && !is_pv
            && (depth <= 7
                || (ctx.allow_null
                    && !is_mate_score(beta)
                    && board.has_non_pawn_material(board.side_to_move())));
        if need_eval && static_eval.is_none() {
            static_eval = Some(evaluate_with_acc(board, &mut self.state.nnue_acc, ctx.ply));
        }
        self.state.eval_stack[ctx.ply] = static_eval;
        let improving = !checked
            && static_eval.is_some_and(|cur| {
                (ctx.ply >= 2 && self.state.eval_stack[ctx.ply - 2].is_some_and(|prev| cur > prev))
                    || (ctx.ply >= 4
                        && self.state.eval_stack[ctx.ply - 2].is_none()
                        && self.state.eval_stack[ctx.ply - 4].is_some_and(|prev| cur > prev))
            });
        let imminent_promo =
            (board.pieces_bb(Color::White, PieceType::Pawn).as_raw() & 0x000F_8000) != 0
                || (board.pieces_bb(Color::Black, PieceType::Pawn).as_raw() & 0x0000_03E0) != 0;
        let prep = MovePruneCtx::prepare(
            depth,
            (alpha, beta),
            (checked, is_pv, !imminent_promo, improving),
            static_eval,
            tt_probe.best_move.is_some(),
            ctx.ply,
        );
        if let Some(score) = prep.rfp_cutoff {
            return score;
        }
        if let Some(score) =
            self.try_null_move(board, prep.depth, beta, is_pv, prep.static_eval, ctx)
        {
            return score;
        }

        self.search_children(
            board,
            &prep,
            tt_probe.best_move,
            (alpha, beta),
            checked,
            ctx,
        )
    }

    #[inline]
    fn try_null_move(
        &mut self,
        board: &Board,
        depth: u8,
        beta: i32,
        is_pv: bool,
        static_eval: Option<i32>,
        ctx: NodeCtx<'_>,
    ) -> Option<i32> {
        let eval = static_eval?;
        if is_pv
            || !ctx.allow_null
            || ctx.ply < self.state.nmp_min_ply
            || depth < 4
            || eval < beta + 20
            || is_mate_score(beta)
            || !board.has_non_pawn_material(board.side_to_move())
        {
            return None;
        }

        let eval_bonus = u8::from(eval - beta >= 180);
        let r = 2 + depth / 5 + eval_bonus;
        let null_depth = depth.saturating_sub(1 + r).max(1);

        let mut null_board = *board;
        null_board.make_null_move();
        self.state.nnue_acc.push_null(ctx.ply + 1);
        let null_ctx = ctx.advance_null(self.state.path_keys.len());
        self.state.path_keys.push(null_board.key());
        let mut null_score = -self.negamax(&null_board, null_depth, -beta, -beta + 1, null_ctx);
        self.state.path_keys.pop();

        if self.state.aborted || null_score < beta {
            return None;
        }
        if is_mate_score(null_score) {
            null_score = beta;
        }
        if depth >= 12 {
            let prev_nmp_min = self.state.nmp_min_ply;
            self.state.nmp_min_ply = ctx.ply + (3 * usize::from(null_depth)) / 4;
            let verify_ctx = NodeCtx {
                allow_null: false,
                ..ctx
            };
            let verified = self.negamax(board, null_depth, beta - 1, beta, verify_ctx);
            self.state.nmp_min_ply = prev_nmp_min;
            if self.state.aborted || verified < beta {
                return None;
            }
            return Some(if is_mate_score(verified) {
                beta
            } else {
                verified
            });
        }
        Some(null_score)
    }

    #[allow(clippy::too_many_arguments)]
    fn search_children(
        &mut self,
        board: &Board,
        prep: &PreparedNode,
        tt_move: Option<Move>,
        (alpha, beta): (i32, i32),
        checked: bool,
        ctx: NodeCtx<'_>,
    ) -> i32 {
        let key = board.key();
        let mut node = NodeSearch::new(alpha);
        if let Some(cutoff) = self.try_tt_move(board, prep, tt_move, beta, checked, ctx, &mut node)
        {
            return cutoff;
        }

        let mut moves = generate_legal_moves(board);
        if moves.is_empty() {
            return if checked { mated_score_in(ctx.ply) } else { 0 };
        }
        if ctx.halfmove >= 100 {
            return 0;
        }
        let start_idx = usize::from(node.skip_move.is_some());
        if let Some(tt_mv) = node.skip_move {
            moves.retain(|&mv| mv != tt_mv);
        }
        let (mut scored, len) = order_moves(board, &moves, None, &self.state, ctx.ply);

        for idx in 0..len {
            let move_idx = start_idx + idx;
            let item = pick_next_move(board, &mut scored, idx, len);
            let mv = item.mv();
            let quiet = item.is_quiet();
            if quiet {
                node.quiet_count += 1;
            }

            let (prunable, lmr_candidate) = prep.prune_ctx.classify(
                board,
                mv,
                move_idx,
                quiet,
                node.quiet_count,
                item.score(),
                node.best_score,
            );
            let checks = (prunable || lmr_candidate) && gives_check(board, mv);
            if prunable && !checks {
                continue;
            }

            let prior_quiets = node.searched_quiets.len();
            if quiet {
                node.searched_quiets.push(mv);
            }

            let red = if lmr_candidate && !checks {
                lmr_reduction(
                    prep.depth,
                    prep.child_depth,
                    move_idx,
                    (beta - alpha > 1, prep.prune_ctx.improving),
                    item.score(),
                )
            } else {
                0
            };
            self.state.nnue_acc.push_move(ctx.ply + 1, board, mv);
            let mut next_board = *board;
            let child_ctx = ctx.advance(&mut next_board, mv, self.state.path_keys.len());
            let score = self.search_window(
                &next_board,
                prep.child_depth,
                red,
                move_idx == 0,
                (node.alpha, beta),
                child_ctx,
            );
            if self.state.aborted {
                return 0;
            }

            if score > node.best_score {
                node.best_score = score;
                node.best_move = Some(mv);
                if score > node.alpha {
                    if score >= beta {
                        self.record_beta_cutoff(
                            board,
                            key,
                            prep.depth,
                            node.best_score,
                            prep.static_eval,
                            &node.searched_quiets.as_slice()[..prior_quiets],
                            mv,
                            ctx.ply,
                        );
                        return node.best_score;
                    }
                    node.alpha = score;
                    self.state.update_pv(ctx.ply, mv);
                }
            }
        }

        self.store_node_result(
            key,
            prep.depth,
            node.best_score,
            node.orig_alpha,
            node.best_move,
            prep.static_eval,
            ctx.ply,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_tt_move(
        &mut self,
        board: &Board,
        prep: &PreparedNode,
        tt_move: Option<Move>,
        beta: i32,
        checked: bool,
        ctx: NodeCtx<'_>,
        node: &mut NodeSearch,
    ) -> Option<i32> {
        let tt_mv = tt_move?;
        if !is_legal_move(board, tt_mv) {
            return None;
        }
        if checked && ctx.halfmove >= 100 {
            return Some(0);
        }
        node.skip_move = Some(tt_mv);
        if is_quiet_move(board, tt_mv) {
            node.quiet_count = 1;
            node.searched_quiets.push(tt_mv);
        }
        self.state.nnue_acc.push_move(ctx.ply + 1, board, tt_mv);
        let mut next_board = *board;
        let child_ctx = ctx.advance(&mut next_board, tt_mv, self.state.path_keys.len());
        let score = self.search_window(
            &next_board,
            prep.child_depth,
            0,
            true,
            (node.alpha, beta),
            child_ctx,
        );
        if self.state.aborted {
            return Some(0);
        }
        node.best_score = score;
        node.best_move = Some(tt_mv);
        if score > node.alpha {
            if score >= beta {
                self.record_beta_cutoff(
                    board,
                    board.key(),
                    prep.depth,
                    node.best_score,
                    prep.static_eval,
                    &[],
                    tt_mv,
                    ctx.ply,
                );
                return Some(node.best_score);
            }
            node.alpha = score;
            self.state.update_pv(ctx.ply, tt_mv);
        }
        None
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn store_node_result(
        &mut self,
        key: u64,
        depth: u8,
        best_score: i32,
        orig_alpha: i32,
        best_move: Option<Move>,
        static_eval: Option<i32>,
        ply: usize,
    ) -> i32 {
        let (bound, stored_move) = if best_score > orig_alpha {
            (Bound::Exact, best_move)
        } else {
            (Bound::Upper, None)
        };
        self.tt
            .store_at(key, depth, bound, best_score, ply, stored_move, static_eval);
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
        static_eval: Option<i32>,
        tried_quiets: &[Move],
        cutoff_move: Move,
        ply: usize,
    ) {
        self.record_quiet_cutoff(board, tried_quiets, cutoff_move, depth, ply);
        self.tt.store_at(
            key,
            depth,
            Bound::Lower,
            best_score,
            ply,
            Some(cutoff_move),
            static_eval,
        );
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
