#![allow(missing_docs)]

use std::time::Instant;
use zest::board::Board;
use zest::movegen::generate_legal_moves;

#[must_use]
pub fn perft(board: &Board, depth: usize) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = generate_legal_moves(board);
    if depth == 1 {
        return moves.len() as u64;
    }

    let mut nodes = 0;
    for m in moves {
        let mut next_board = *board;
        next_board.make_move(m);
        nodes += perft(&next_board, depth - 1);
    }
    nodes
}

#[test]
fn test_perft_depth_1_to_6() {
    let board = Board::gardner_initial();

    let start = Instant::now();
    let d1 = perft(&board, 1);
    let d2 = perft(&board, 2);
    let d3 = perft(&board, 3);
    let d4 = perft(&board, 4);
    let d5 = perft(&board, 5);
    let d6 = perft(&board, 6);
    let elapsed = start.elapsed();

    println!("Perft results for Gardner Minichess starting position:");
    println!("  Depth 1: {d1}");
    println!("  Depth 2: {d2}");
    println!("  Depth 3: {d3}");
    println!("  Depth 4: {d4}");
    println!("  Depth 5: {d5}");
    println!("  Depth 6: {d6}");
    println!("  Total time (d1-d6): {elapsed:?}");

    assert_eq!(d1, 7);
    assert_eq!(d2, 53);
    assert_eq!(d3, 506);
    assert_eq!(d4, 4775);
    assert_eq!(d5, 52_512);
    assert_eq!(d6, 572_874);
}
