use crate::board::Board;
use crate::eval::evaluate;
use crate::movegen::{generate_legal_moves, in_check};
use crate::search::ordering::{order_moves, order_tactical_moves, pick_next_move};
use crate::search::state::{NodeCtx, SearchState};
use crate::search::tt::TranspositionTable;
use crate::search::tt_entry::Bound;
use crate::search::types::{INF_SCORE, MAX_PLY, mated_score_in};

pub(super) fn quiesce(
    state: &mut SearchState,
    tt: &mut TranspositionTable,
    board: &Board,
    mut alpha: i32,
    beta: i32,
    ctx: NodeCtx<'_>,
) -> i32 {
    if state.check_limits(ctx.stop) {
        return 0;
    }

    state.nodes += 1;

    if ctx.ply >= MAX_PLY {
        return evaluate(board);
    }

    let key = board.key();
    if state.is_repetition(key, ctx.reversible_start) || board.is_insufficient_material() {
        return 0;
    }

    let tt_probe = tt.probe_cutoff(key, 0, ctx.ply, alpha, beta, ctx.halfmove);
    if let Some((tt_score, _)) = tt_probe.cutoff {
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

    let orig_alpha = alpha;
    let mut best_score = -INF_SCORE;
    let mut best_move = None;

    if !checked {
        let stand_pat = evaluate(board);
        if stand_pat >= beta {
            tt.store_at(key, 0, Bound::Lower, stand_pat, ctx.ply, None);
            return stand_pat;
        }
        best_score = stand_pat;
        if stand_pat > alpha {
            alpha = stand_pat;
        }
    }

    let (mut scored, len) = if checked {
        order_moves(board, &moves, tt_probe.best_move, state, ctx.ply)
    } else {
        order_tactical_moves(board, &moves, tt_probe.best_move)
    };

    for idx in 0..len {
        let mv = pick_next_move(&mut scored, idx, len);
        let mut next_board = *board;
        let child_ctx = ctx.advance(&mut next_board, mv, state.path_keys.len());

        state.path_keys.push(next_board.key());
        let score = -quiesce(state, tt, &next_board, -beta, -alpha, child_ctx);
        state.path_keys.pop();

        if state.aborted {
            return 0;
        }

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
            if score > alpha {
                if score >= beta {
                    tt.store_at(key, 0, Bound::Lower, best_score, ctx.ply, best_move);
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
    tt.store_at(key, 0, bound, best_score, ctx.ply, stored_move);
    best_score
}
