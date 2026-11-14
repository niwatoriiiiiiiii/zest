use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::core::Move;
use crate::eval::evaluate;
use crate::movegen::{MoveList, generate_legal_moves, in_check};
use crate::search::ordering::{order_moves, pick_next_move};
use crate::search::state::{SearchState, advance_board_and_clock};
use crate::search::types::{INF_SCORE, MAX_PLY, SearchLimits, SearchResult, mated_score_in};

/// Stateful game-tree search engine using iterative deepening and negamax alpha-beta pruning.
#[derive(Debug)]
pub struct Searcher {
    state: SearchState,
}

impl Default for Searcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Searcher {
    /// Creates a reusable [`Searcher`] with pre-allocated working buffers.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: SearchState::new(),
        }
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
                pv: Vec::new(),
            };
        }

        self.state.reset(board.key(), history, limits.max_nodes);

        let fallback_move = root_moves[0];
        let mut best_move = fallback_move;
        let mut best_score = evaluate(board);
        let mut completed_depth = 0u8;
        let mut best_pv = vec![fallback_move];
        let mut pv_hint = Some(fallback_move);

        let max_depth = usize::from(limits.max_depth.unwrap_or(64)).clamp(1, MAX_PLY);

        for depth in 1..=max_depth {
            if self.state.should_stop(stop) {
                break;
            }

            #[allow(clippy::cast_possible_truncation)]
            let score = self.negamax_root(
                board,
                &root_moves,
                depth as u8,
                halfmove_clock,
                pv_hint,
                stop,
            );
            if self.state.aborted {
                break;
            }

            let root_len = self.state.pv_len[0];
            if root_len > 0 {
                let candidate = self.state.pv_table[0][0];
                best_move = candidate;
                best_score = score;
                #[allow(clippy::cast_possible_truncation)]
                {
                    completed_depth = depth as u8;
                }
                best_pv = self.state.pv_table[0][..root_len].to_vec();
                pv_hint = Some(candidate);
            }
        }

        SearchResult {
            best_move: Some(best_move),
            score: best_score,
            depth: completed_depth,
            nodes: self.state.nodes,
            pv: best_pv,
        }
    }

    fn negamax_root(
        &mut self,
        board: &Board,
        moves: &MoveList,
        depth: u8,
        halfmove_clock: u16,
        pv_hint: Option<Move>,
        stop: Option<&AtomicBool>,
    ) -> i32 {
        self.state.nodes += 1;
        self.state.pv_len[0] = 0;

        let mut alpha = -INF_SCORE;
        let beta = INF_SCORE;
        let (mut scored, len) = order_moves(board, moves, pv_hint);

        for idx in 0..len {
            let mv = pick_next_move(&mut scored, idx, len);
            let mut next_board = *board;
            let (next_halfmove, next_rev_start) = advance_board_and_clock(
                &mut next_board,
                mv,
                halfmove_clock,
                0,
                self.state.path_keys.len(),
            );

            self.state.path_keys.push(next_board.key());
            let score = -self.negamax(
                &next_board,
                depth - 1,
                1,
                -beta,
                -alpha,
                next_halfmove,
                next_rev_start,
                stop,
            );
            self.state.path_keys.pop();

            if self.state.aborted {
                return 0;
            }

            if score > alpha {
                alpha = score;
                self.state.update_pv(0, mv);
            }
        }

        alpha
    }

    #[allow(clippy::too_many_arguments)]
    fn negamax(
        &mut self,
        board: &Board,
        depth: u8,
        ply: usize,
        mut alpha: i32,
        beta: i32,
        halfmove_clock: u16,
        reversible_start: usize,
        stop: Option<&AtomicBool>,
    ) -> i32 {
        self.state.pv_len[ply] = ply;

        if self.state.check_limits(stop) {
            return 0;
        }

        self.state.nodes += 1;

        if ply >= MAX_PLY {
            return evaluate(board);
        }

        if self.state.is_repetition(board.key(), reversible_start)
            || board.is_insufficient_material()
        {
            return 0;
        }

        let moves = generate_legal_moves(board);
        if moves.is_empty() {
            return if in_check(board) {
                mated_score_in(ply)
            } else {
                0
            };
        }

        if halfmove_clock >= 100 {
            return 0;
        }

        if depth == 0 {
            return evaluate(board);
        }

        let (mut scored, len) = order_moves(board, &moves, None);
        for idx in 0..len {
            let mv = pick_next_move(&mut scored, idx, len);
            let mut next_board = *board;
            let (next_halfmove, next_rev_start) = advance_board_and_clock(
                &mut next_board,
                mv,
                halfmove_clock,
                reversible_start,
                self.state.path_keys.len(),
            );

            self.state.path_keys.push(next_board.key());
            let score = -self.negamax(
                &next_board,
                depth - 1,
                ply + 1,
                -beta,
                -alpha,
                next_halfmove,
                next_rev_start,
                stop,
            );
            self.state.path_keys.pop();

            if self.state.aborted {
                return 0;
            }

            if score >= beta {
                return beta;
            }

            if score > alpha {
                alpha = score;
                self.state.update_pv(ply, mv);
            }
        }

        alpha
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
