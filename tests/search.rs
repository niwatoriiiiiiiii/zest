#![allow(missing_docs)]

use core::sync::atomic::AtomicBool;

use zest::board::Board;
use zest::core::Move;
use zest::movegen::{generate_legal_moves, in_check};
use zest::search::{
    MATE_SCORE, SearchLimits, Searcher, is_mate_score, mate_distance_in_moves, search,
};

fn board_from_fen(fen: &str) -> Board {
    Board::from_fen(fen).expect("valid test FEN")
}

fn uci(s: &str) -> Move {
    Move::from_uci(s).expect("valid UCI move")
}

#[test]
fn terminal_checkmate_and_stalemate_at_root() {
    let mate_board = board_from_fen("4k/4Q/4K/5/5 b - - 0 1");
    let mate_res = search(&mate_board, &[], 0, SearchLimits::depth(2));
    assert_eq!(mate_res.best_move, None);
    assert_eq!(mate_res.score, -MATE_SCORE);
    assert_eq!(mate_res.depth, 0);
    assert!(mate_res.pv.is_empty());

    let stale_board = board_from_fen("4k/5/3QK/5/5 b - - 0 1");
    let stale_res = search(&stale_board, &[], 0, SearchLimits::depth(2));
    assert_eq!(stale_res.best_move, None);
    assert_eq!(stale_res.score, 0);
    assert_eq!(stale_res.depth, 0);
    assert!(stale_res.pv.is_empty());
}

#[test]
fn finds_mate_in_one() {
    let board = board_from_fen("4k/5/4K/5/2R2 w - - 0 1");
    let res = search(&board, &[], 0, SearchLimits::depth(3));

    assert_eq!(res.best_move, Some(uci("c1c5")));
    assert!(is_mate_score(res.score));
    assert_eq!(mate_distance_in_moves(res.score), Some(1));
    assert_eq!(res.pv, vec![uci("c1c5")]);
}

#[test]
fn finds_forced_mate_in_two() {
    let board = board_from_fen("k3r/pp3/2b2/4R/2K1Q w - - 0 1");
    let res = search(&board, &[], 0, SearchLimits::depth(4));

    assert_eq!(res.best_move, Some(uci("e2e5")));
    assert!(is_mate_score(res.score));
    assert_eq!(mate_distance_in_moves(res.score), Some(2));
    assert_eq!(res.pv, vec![uci("e2e5"), uci("c3e5"), uci("e1e5")]);
}

#[test]
fn quiescence_avoids_poisoned_capture_at_depth_one() {
    let board = board_from_fen("1r1k1/5/1p3/5/1Q2K w - - 0 1");
    let res = search(&board, &[], 0, SearchLimits::depth(1));

    assert_ne!(res.best_move, Some(uci("b1b3")));
    assert!(res.score > 0, "expected positive score, got {}", res.score);
}

#[test]
fn quiescence_resolves_capture_chain_to_find_winning_exchange() {
    let board = board_from_fen("2r1k/5/2b2/1Q3/2RK1 w - - 0 1");
    let res = search(&board, &[], 0, SearchLimits::depth(1));

    assert_eq!(res.best_move, Some(uci("c1c3")));
    assert!(
        res.score > 400,
        "expected winning exchange score, got {}",
        res.score
    );
}

#[test]
fn avoids_stalemate_when_winning() {
    let board = board_from_fen("4k/5/3QK/5/5 w - - 0 1");
    let res = search(&board, &[], 0, SearchLimits::depth(2));

    let best = res.best_move.expect("expected a winning move");
    let mut next = board;
    next.make_move(best);
    assert!(
        !generate_legal_moves(&next).is_empty() || in_check(&next),
        "winning side should not play a stalemate move: {best}"
    );
    assert!(is_mate_score(res.score));
}

#[test]
fn repetition_draw_detection_via_history() {
    let mut b = board_from_fen("1k3/5/5/3R1/3QK b - - 0 1");
    let k0 = b.key();
    b.make_move(uci("b5a5"));
    let k1 = b.key();
    b.make_move(uci("d2e2"));
    let k2 = b.key();
    b.make_move(uci("a5b5"));
    let k3 = b.key();
    b.make_move(uci("e2d2"));

    let mut searcher = Searcher::new();
    let res_losing = searcher.search(&b, &[k0, k1, k2, k3], 4, SearchLimits::depth(2));
    assert_eq!(res_losing.best_move, Some(uci("b5a5")));
    assert_eq!(res_losing.score, 0);
}

#[test]
fn draws_on_insufficient_material_and_fifty_move_clock() {
    let insufficient = board_from_fen("4k/5/2B2/5/4K w - - 0 1");
    let res_insuff = search(&insufficient, &[], 0, SearchLimits::depth(3));
    assert_eq!(res_insuff.score, 0);

    let board = board_from_fen("4k/5/5/5/1Q2K w - - 0 1");
    let res_fifty = search(&board, &[], 100, SearchLimits::depth(2));
    assert_eq!(res_fifty.score, 0);
}

#[test]
fn respects_node_limit_and_stop_flag() {
    let board = Board::gardner_initial();
    let mut searcher = Searcher::new();

    let node_limited = searcher.search(&board, &[], 0, SearchLimits::nodes(50));
    assert!(node_limited.best_move.is_some());
    assert!(node_limited.nodes <= 64);

    let stop = AtomicBool::new(true);
    let stopped = searcher.search_with_stop(&board, &[], 0, SearchLimits::depth(6), &stop);
    assert!(stopped.best_move.is_some());
    assert_eq!(stopped.depth, 0);
}

#[test]
fn iterative_deepening_produces_legal_pv() {
    let board = Board::gardner_initial();
    let res = search(&board, &[], 0, SearchLimits::depth(3));

    assert_eq!(res.depth, 3);
    assert!(!res.pv.is_empty());
    assert_eq!(res.best_move, Some(res.pv[0]));

    let mut cur = board;
    for mv in &res.pv {
        let legal = generate_legal_moves(&cur);
        assert!(
            legal.as_slice().contains(mv),
            "PV move {mv} must be legal in current position"
        );
        cur.make_move(*mv);
    }
}

#[test]
fn tt_reduces_nodes_and_clear_resets_deterministically() {
    let board = Board::gardner_initial();
    let mut searcher = Searcher::new();

    let first = searcher.search(&board, &[], 0, SearchLimits::depth(6));
    let second = searcher.search(&board, &[], 0, SearchLimits::depth(6));

    assert!(
        second.nodes < first.nodes / 2,
        "warm TT should prune repeated depth-6 search ({} vs {})",
        second.nodes,
        first.nodes
    );
    assert_eq!(second.best_move, first.best_move);
    assert_eq!(second.score, first.score);

    searcher.clear_tt();
    let after_clear = searcher.search(&board, &[], 0, SearchLimits::depth(6));
    assert_eq!(after_clear.nodes, first.nodes);
    assert_eq!(after_clear.best_move, first.best_move);
}

#[test]
fn tt_preserves_mate_distance_across_root_and_child_plies() {
    let root = board_from_fen("k3r/pp3/2b2/4R/2K1Q w - - 0 1");
    let mut after_sac = root;
    after_sac.make_move(uci("e2e5"));

    let mut searcher = Searcher::new();
    let res_root = searcher.search(&root, &[], 0, SearchLimits::depth(4));
    assert_eq!(res_root.best_move, Some(uci("e2e5")));
    assert_eq!(mate_distance_in_moves(res_root.score), Some(2));

    let res_child = searcher.search(&after_sac, &[], 0, SearchLimits::depth(3));
    assert_eq!(mate_distance_in_moves(res_child.score), Some(-1));
    assert!(
        res_child.pv.len() >= 2,
        "expected extended PV across TT hit, got {:?}",
        res_child.pv
    );
}

#[test]
fn repetition_and_fifty_move_precedence_hold_with_warm_tt() {
    let mut b = board_from_fen("1k3/5/5/3R1/3QK b - - 0 1");
    let k0 = b.key();
    b.make_move(uci("b5a5"));
    let k1 = b.key();
    b.make_move(uci("d2e2"));
    let k2 = b.key();
    b.make_move(uci("a5b5"));
    let k3 = b.key();
    b.make_move(uci("e2d2"));

    let mut searcher = Searcher::new();
    let non_repeating = searcher.search(&b, &[], 0, SearchLimits::depth(3));
    assert_ne!(non_repeating.score, 0);

    let repeating = searcher.search(&b, &[k0, k1, k2, k3], 4, SearchLimits::depth(2));
    assert_eq!(repeating.best_move, Some(uci("b5a5")));
    assert_eq!(repeating.score, 0);

    let quiet_board = board_from_fen("4k/5/5/5/1Q2K w - - 0 1");
    let warm_eval = searcher.search(&quiet_board, &[], 0, SearchLimits::depth(3));
    assert!(warm_eval.score > 500);
    let fifty_draw = searcher.search(&quiet_board, &[], 100, SearchLimits::depth(2));
    assert_eq!(fifty_draw.score, 0);
}

#[test]
fn pvs_and_aspiration_reduce_startpos_nodes_without_altering_pv() {
    let board = Board::gardner_initial();
    let mut searcher = Searcher::new();
    let res = searcher.search(&board, &[], 0, SearchLimits::depth(7));

    assert_eq!(res.depth, 7);
    assert_eq!(res.score, 8);
    assert_eq!(res.best_move, Some(uci("a2a3")));
    assert_eq!(
        res.pv,
        vec![
            uci("a2a3"),
            uci("b4a3"),
            uci("b2a3"),
            uci("e4e3"),
            uci("d2e3"),
            uci("d4e3"),
            uci("d1d5"),
        ]
    );
    assert!(
        res.nodes < 23_500,
        "expected PVS + killer/history + aspiration to prune startpos depth 7 below 23500 nodes, got {}",
        res.nodes
    );
}

#[test]
fn aspiration_recovers_cleanly_on_tactical_mate_swing() {
    let board = board_from_fen("k3r/pp3/2b2/4R/2K1Q w - - 0 1");
    let mut searcher = Searcher::new();
    let res = searcher.search(&board, &[], 0, SearchLimits::depth(5));

    assert_eq!(res.depth, 5);
    assert_eq!(res.best_move, Some(uci("e2e5")));
    assert_eq!(mate_distance_in_moves(res.score), Some(2));
}
