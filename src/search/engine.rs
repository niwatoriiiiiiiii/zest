use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::core::Move;
use crate::eval::evaluate;
use crate::movegen::{MoveList, generate_legal_moves, in_check};
use crate::search::ordering::{ScoredMove, order_moves, pick_next_move};
use crate::search::quiescence::quiesce;
use crate::search::state::{NodeCtx, SearchState, is_quiet_move};
use crate::search::tt::{DEFAULT_HASH_MB, TranspositionTable};
use crate::search::tt_entry::Bound;
use crate::search::types::{
    INF_SCORE, MAX_PLY, SearchLimits, SearchResult, is_mate_score, mated_score_in,
};

const ASPIRATION_MIN_DEPTH: usize = 4;
const ASPIRATION_INITIAL_DELTA: i32 = 50;
const ASPIRATION_MAX_DELTA: i32 = 400;

/// Stateful game-tree search engine using iterative deepening and PVS alpha-beta pruning.
#[derive(Debug)]
pub struct Searcher {
    state: SearchState,
    tt: TranspositionTable,
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

    fn search_inner(
        &mut self,
        board: &Board,
        history: &[u64],
        halfmove_clock: u16,
        limits: SearchLimits,
        stop: Option<&AtomicBool>,
    ) -> SearchResult {
        let root_moves = generate_legal_moves(board);
        if root_moves.is_empty() {
            let terminal_score = if in_check(board) {
                mated_score_in(0)
            } else {
                0
            };
            return SearchResult {
                best_move: None,
                score: terminal_score,
                depth: 0,
                nodes: 1,
                hashfull: 0,
                pv: Vec::new(),
            };
        }

        self.state.reset(board.key(), history, limits.max_nodes);
        self.tt.new_search();

        let fallback_move = root_moves[0];
        let mut best_move = fallback_move;
        let mut best_score = evaluate(board);
        let mut completed_depth = 0u8;
        let mut best_pv = vec![fallback_move];
        let mut pv_hint = None;

        let max_depth = usize::from(limits.max_depth.unwrap_or(64)).clamp(1, MAX_PLY);
        let root_ctx = NodeCtx {
            ply: 0,
            halfmove: halfmove_clock,
            reversible_start: 0,
            stop,
        };

        for depth in 1..=max_depth {
            if self.state.should_stop(stop) {
                break;
            }

            #[allow(clippy::cast_possible_truncation)]
            let depth_u8 = depth as u8;
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
        }

        SearchResult {
            best_move: Some(best_move),
            score: best_score,
            depth: completed_depth,
            nodes: self.state.nodes,
            hashfull: self.tt.hashfull_permille(),
            pv: best_pv,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn search_aspiration(
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

    fn extend_pv_from_tt(&self, root: &Board, pv: &mut Vec<Move>, max_len: usize) {
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
            let mv = pick_next_move(&mut scored, idx, len);
            let score = self.search_pvs_child(board, mv, depth, idx, alpha, beta, ctx);
            if self.state.aborted {
                return 0;
            }
            if score > best_score {
                best_score = score;
                best_move = Some(mv);
                if score > alpha {
                    if score >= beta {
                        self.state.update_pv(0, mv);
                        self.tt
                            .store_at(key, depth, Bound::Lower, best_score, 0, best_move);
                        return best_score;
                    }
                    alpha = score;
                    self.state.update_pv(0, mv);
                }
            }
        }

        let (bound, stored_move) = if best_score > orig_alpha {
            (Bound::Exact, best_move)
        } else {
            (Bound::Upper, None)
        };
        self.tt
            .store_at(key, depth, bound, best_score, 0, stored_move);
        best_score
    }

    fn negamax(
        &mut self,
        board: &Board,
        depth: u8,
        mut alpha: i32,
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

        let tt_probe = self
            .tt
            .probe_cutoff(key, depth, ctx.ply, alpha, beta, ctx.halfmove);
        if let Some((tt_score, is_exact)) = tt_probe.cutoff {
            if is_exact && let Some(mv) = tt_probe.best_move {
                self.state.pv_table[ctx.ply][ctx.ply] = mv;
                self.state.pv_len[ctx.ply] = ctx.ply + 1;
            }
            return tt_score;
        }

        let moves = generate_legal_moves(board);
        if moves.is_empty() {
            return if in_check(board) {
                mated_score_in(ctx.ply)
            } else {
                0
            };
        }
        if ctx.halfmove >= 100 {
            return 0;
        }

        let orig_alpha = alpha;
        let mut best_score = -INF_SCORE;
        let mut best_move = None;
        let (mut scored, len) =
            order_moves(board, &moves, tt_probe.best_move, &self.state, ctx.ply);

        for idx in 0..len {
            let mv = pick_next_move(&mut scored, idx, len);
            let score = self.search_pvs_child(board, mv, depth, idx, alpha, beta, ctx);
            if self.state.aborted {
                return 0;
            }
            if score > best_score {
                best_score = score;
                best_move = Some(mv);
                if score > alpha {
                    if score >= beta {
                        self.record_quiet_cutoff(board, &scored[..idx], mv, depth, ctx.ply);
                        self.tt
                            .store_at(key, depth, Bound::Lower, best_score, ctx.ply, best_move);
                        return best_score;
                    }
                    alpha = score;
                    self.state.update_pv(ctx.ply, mv);
                }
            }
        }

        let (bound, stored_move) = if best_score > orig_alpha {
            (Bound::Exact, best_move)
        } else {
            (Bound::Upper, None)
        };
        self.tt
            .store_at(key, depth, bound, best_score, ctx.ply, stored_move);
        best_score
    }

    #[inline]
    fn record_quiet_cutoff(
        &mut self,
        board: &Board,
        tried: &[ScoredMove],
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
        for item in tried {
            let prev = item.mv();
            if is_quiet_move(board, prev) {
                self.state.update_history(side, prev, -bonus);
            }
        }
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn search_pvs_child(
        &mut self,
        board: &Board,
        mv: Move,
        depth: u8,
        move_idx: usize,
        alpha: i32,
        beta: i32,
        ctx: NodeCtx<'_>,
    ) -> i32 {
        let mut next_board = *board;
        let child_ctx = ctx.advance(&mut next_board, mv, self.state.path_keys.len());
        self.state.path_keys.push(next_board.key());

        let raw = if move_idx == 0 {
            -self.negamax(&next_board, depth - 1, -beta, -alpha, child_ctx)
        } else {
            let null_score = -self.negamax(&next_board, depth - 1, -alpha - 1, -alpha, child_ctx);
            if !self.state.aborted && null_score > alpha && null_score < beta {
                -self.negamax(&next_board, depth - 1, -beta, -alpha, child_ctx)
            } else {
                null_score
            }
        };

        // Ensure strict fail-soft return even if negated bounds collapsed on abort.
        let score = if self.state.aborted { 0 } else { raw };
        self.state.path_keys.pop();
        score
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
