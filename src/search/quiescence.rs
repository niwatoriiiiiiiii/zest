use crate::board::Board;
use crate::eval::evaluate;
use crate::movegen::{generate_legal_moves, generate_tactical_moves, gives_check, in_check};
use crate::search::ordering::{
    mvv_value, order_moves, order_tactical_moves, pick_next_move, pick_next_scored,
};
use crate::search::state::{NodeCtx, SearchState};
use crate::search::tt::TranspositionTable;
use crate::search::tt_entry::Bound;
use crate::search::types::{INF_SCORE, MAX_PLY, is_mate_score, mated_score_in};

#[inline]
fn can_delta_prune(board: &Board, mv: crate::core::Move, stand_pat: i32, alpha: i32) -> bool {
    if mv.promotion().is_some() || is_mate_score(alpha) || gives_check(board, mv) {
        return false;
    }
    let gain = board
        .piece_at(mv.to())
        .map_or(0, |p| mvv_value(p.piece_type));
    stand_pat + gain + 140 < alpha
}

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
    let orig_alpha = alpha;
    let mut best_score = -INF_SCORE;
    let mut best_move = None;
    let mut static_eval = None;

    let (mut scored, len) = if checked {
        let moves = generate_legal_moves(board);
        if moves.is_empty() {
            return mated_score_in(ctx.ply);
        }
        if ctx.halfmove >= 100 {
            return 0;
        }
        order_moves(board, &moves, tt_probe.best_move, state, ctx.ply)
    } else {
        if ctx.halfmove >= 100 {
            return 0;
        }
        let stand_pat = tt_probe.static_eval.unwrap_or_else(|| evaluate(board));
        static_eval = Some(stand_pat);
        if stand_pat >= beta {
            tt.store_at(key, 0, Bound::Lower, stand_pat, ctx.ply, None, static_eval);
            return stand_pat;
        }
        best_score = stand_pat;
        if stand_pat > alpha {
            alpha = stand_pat;
        }
        let moves = generate_tactical_moves(board);
        order_tactical_moves(board, &moves, tt_probe.best_move)
    };

    for idx in 0..len {
        let item = if checked {
            pick_next_move(board, &mut scored, idx, len)
        } else {
            pick_next_scored(&mut scored, idx, len)
        };
        let mv = item.mv();
        if let Some(stand_pat) = static_eval
            && can_delta_prune(board, mv, stand_pat, alpha)
        {
            continue;
        }
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
                    tt.store_at(
                        key,
                        0,
                        Bound::Lower,
                        best_score,
                        ctx.ply,
                        best_move,
                        static_eval,
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
    tt.store_at(key, 0, bound, best_score, ctx.ply, stored_move, static_eval);
    best_score
}
