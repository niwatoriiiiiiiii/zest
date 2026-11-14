use core::sync::atomic::AtomicBool;

use crate::board::Board;
use crate::eval::evaluate;
use crate::movegen::{generate_legal_moves, in_check};
use crate::search::ordering::{order_moves, order_tactical_moves, pick_next_move};
use crate::search::state::{SearchState, advance_board_and_clock};
use crate::search::types::{MAX_PLY, mated_score_in};

#[allow(clippy::too_many_arguments)]
pub(super) fn quiesce(
    state: &mut SearchState,
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

    if state.is_repetition(board.key(), reversible_start) || board.is_insufficient_material() {
        return 0;
    }

    let checked = in_check(board);
    let moves = generate_legal_moves(board);
    if moves.is_empty() {
        return if checked { mated_score_in(ply) } else { 0 };
    }

    if halfmove_clock >= 100 {
        return 0;
    }

    if !checked {
        let stand_pat = evaluate(board);
        if stand_pat >= beta {
            return beta;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }
    }

    let (mut scored, len) = if checked {
        order_moves(board, &moves, None)
    } else {
        order_tactical_moves(board, &moves)
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

        if score >= beta {
            return beta;
        }

        if score > alpha {
            alpha = score;
        }
    }

    alpha
}
