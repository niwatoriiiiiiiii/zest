use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::core::Move;
use crate::eval::evaluate;
use crate::movegen::{generate_legal_moves, in_check};
use crate::search::ordering::{order_moves, order_tactical_moves, pick_next_move};
use crate::search::state::{SearchState, advance_board_and_clock};
use crate::search::tt::{Bound, TranspositionTable, score_from_tt, score_to_tt};
use crate::search::types::{INF_SCORE, MAX_PLY, mated_score_in};

#[inline]
fn probe_tt_quiesce(
    tt: &TranspositionTable,
    key: u64,
    ply: usize,
    alpha: i32,
    beta: i32,
    halfmove_clock: u16,
) -> (Option<i32>, Option<Move>) {
    let Some(hit) = tt.probe(key) else {
        return (None, None);
    };
    let tt_move = hit.best_move;
    if halfmove_clock >= 100 {
        return (None, tt_move);
    }

    let tt_score = score_from_tt(hit.score, ply);
    let cutoff = match hit.bound {
        Bound::Exact => true,
        Bound::Lower => tt_score >= beta,
        Bound::Upper => tt_score <= alpha,
        Bound::None => false,
    };
    if cutoff {
        (Some(tt_score), tt_move)
    } else {
        (None, tt_move)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn quiesce(
    state: &mut SearchState,
    tt: &mut TranspositionTable,
    board: &Board,
    ply: usize,
    mut alpha: i32,
    beta: i32,
    halfmove_clock: u16,
    reversible_start: usize,
    stop: Option<&AtomicBool>,
) -> i32 {
    if state.check_limits(stop) {
        return 0;
    }

    state.nodes += 1;

    if ply >= MAX_PLY {
        return evaluate(board);
    }

    let key = board.key();
    if state.is_repetition(key, reversible_start) || board.is_insufficient_material() {
        return 0;
    }

    let (tt_cutoff, tt_move) = probe_tt_quiesce(tt, key, ply, alpha, beta, halfmove_clock);
    if let Some(tt_score) = tt_cutoff {
        return tt_score;
    }

    let checked = in_check(board);
    let moves = generate_legal_moves(board);
    if moves.is_empty() {
        return if checked { mated_score_in(ply) } else { 0 };
    }

    if halfmove_clock >= 100 {
        return 0;
    }

    let orig_alpha = alpha;
    let mut best_score = -INF_SCORE;
    let mut best_move = None;

    if !checked {
        let stand_pat = evaluate(board);
        if stand_pat >= beta {
            tt.store(key, 0, Bound::Lower, score_to_tt(stand_pat, ply), None);
            return stand_pat;
        }
        best_score = stand_pat;
        if stand_pat > alpha {
            alpha = stand_pat;
        }
    }

    let (mut scored, len) = if checked {
        order_moves(board, &moves, tt_move)
    } else {
        order_tactical_moves(board, &moves, tt_move)
    };

    for idx in 0..len {
        let mv = pick_next_move(&mut scored, idx, len);
        let mut next_board = *board;
        let (next_halfmove, next_rev_start) = advance_board_and_clock(
            &mut next_board,
            mv,
            halfmove_clock,
            reversible_start,
            state.path_keys.len(),
        );

        state.path_keys.push(next_board.key());
        let score = -quiesce(
            state,
            tt,
            &next_board,
            ply + 1,
            -beta,
            -alpha,
            next_halfmove,
            next_rev_start,
            stop,
        );
        state.path_keys.pop();

        if state.aborted {
            return 0;
        }

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
            if score > alpha {
                if score >= beta {
                    tt.store(
                        key,
                        0,
                        Bound::Lower,
                        score_to_tt(best_score, ply),
                        best_move,
                    );
                    return best_score;
                }
                alpha = score;
            }
        }
    }

    let (bound, stored_move) = if best_score > orig_alpha {
        (Bound::Exact, best_move)
    } else {
        (Bound::Upper, None)
    };
    tt.store(key, 0, bound, score_to_tt(best_score, ply), stored_move);

    best_score
}
