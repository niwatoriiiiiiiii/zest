use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::core::{Color, Move, PieceType};
use crate::eval::evaluate_with_acc;
use crate::movegen::in_check;
use crate::search::pruning::MovePruneCtx;
use crate::search::quiescence::quiesce;
use crate::search::state::{NodeCtx, SearchState};
use crate::search::tt::{DEFAULT_HASH_MB, TranspositionTable};
use crate::search::tt_entry::Bound;
use crate::search::types::{MAX_PLY, SearchLimits, SearchResult, is_mate_score};

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

    pub(super) fn negamax(
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
