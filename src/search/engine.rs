use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::core::Move;
use crate::eval::evaluate;
use crate::movegen::{MoveList, generate_legal_moves, in_check};
use crate::search::ordering::{order_moves, pick_next_move};
use crate::search::quiescence::quiesce;
use crate::search::state::{NodeCtx, SearchState};
use crate::search::tt::{DEFAULT_HASH_MB, TranspositionTable};
use crate::search::tt_entry::Bound;
use crate::search::types::{INF_SCORE, MAX_PLY, SearchLimits, SearchResult, mated_score_in};

/// Stateful game-tree search engine using iterative deepening and negamax alpha-beta pruning.
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

    /// Clears all entries from the transposition table.
    pub fn clear_tt(&mut self) {
        self.tt.clear();
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
            let score = self.negamax_root(board, &root_moves, depth_u8, pv_hint, root_ctx);
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

    fn negamax_root(
        &mut self,
        board: &Board,
        moves: &MoveList,
        depth: u8,
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

        let mut alpha = -INF_SCORE;
        let mut best_move = None;
        let (mut scored, len) = order_moves(board, moves, priority_move);

        for idx in 0..len {
            let mv = pick_next_move(&mut scored, idx, len);
            let score = self.search_child(board, mv, depth, alpha, INF_SCORE, ctx);
            if self.state.aborted {
                return 0;
            }
            if score > alpha {
                alpha = score;
                best_move = Some(mv);
                self.state.update_pv(0, mv);
            }
        }

        self.tt
            .store_at(key, depth, Bound::Exact, alpha, 0, best_move);
        alpha
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
        let (mut scored, len) = order_moves(board, &moves, tt_probe.best_move);

        for idx in 0..len {
            let mv = pick_next_move(&mut scored, idx, len);
            let score = self.search_child(board, mv, depth, alpha, beta, ctx);
            if self.state.aborted {
                return 0;
            }
            if score > best_score {
                best_score = score;
                best_move = Some(mv);
                if score > alpha {
                    if score >= beta {
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
    fn search_child(
        &mut self,
        board: &Board,
        mv: Move,
        depth: u8,
        alpha: i32,
        beta: i32,
        ctx: NodeCtx<'_>,
    ) -> i32 {
        let mut next_board = *board;
        let child_ctx = ctx.advance(&mut next_board, mv, self.state.path_keys.len());
        self.state.path_keys.push(next_board.key());
        let score = -self.negamax(&next_board, depth - 1, -beta, -alpha, child_ctx);
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
