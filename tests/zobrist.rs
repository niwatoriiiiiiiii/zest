#![allow(missing_docs)]

use std::collections::HashSet;

use zest::board::{Board, ZobristKeys};
use zest::core::{Color, Move, Piece, PieceType, Square};
use zest::movegen::generate_legal_moves;

fn recompute_key_from_scratch(board: &Board) -> u64 {
    let mut key = 0_u64;
    for &sq in &Square::ALL {
        if let Some(piece) = board.piece_at(sq) {
            key ^= ZobristKeys::piece_key(piece, sq);
        }
    }
    if board.side_to_move() == Color::Black {
        key ^= ZobristKeys::side_to_move_key();
    }
    key
}

#[test]
fn test_zobrist_keys_nonzero_and_unique() {
    let colors = [Color::White, Color::Black];
    let piece_types = [
        PieceType::Pawn,
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
        PieceType::King,
    ];

    let mut keys = HashSet::new();
    for color in colors {
        for piece_type in piece_types {
            let piece = Piece::new(color, piece_type);
            for &sq in &Square::ALL {
                let key = ZobristKeys::piece_key(piece, sq);
                assert_ne!(key, 0, "Piece key should be non-zero");
                assert!(keys.insert(key), "Duplicate piece key detected");
            }
        }
    }

    let side_key = ZobristKeys::side_to_move_key();
    assert_ne!(side_key, 0, "Side-to-move key should be non-zero");
    assert!(
        keys.insert(side_key),
        "Side-to-move key collided with piece key"
    );
    assert_eq!(keys.len(), 301);
}

#[test]
fn test_empty_board_key_is_zero() {
    let board = Board::empty();
    assert_eq!(board.key(), 0);
    assert_eq!(board.key(), recompute_key_from_scratch(&board));
}

#[test]
fn test_incremental_key_matches_full_recomputation() {
    let mut board = Board::empty();

    // Place, overwrite, and remove pieces
    let c3 = Square::C3;
    board.set_piece(c3, Some(Piece::new(Color::White, PieceType::Queen)));
    assert_eq!(board.key(), recompute_key_from_scratch(&board));

    board.set_piece(c3, Some(Piece::new(Color::Black, PieceType::Knight)));
    assert_eq!(board.key(), recompute_key_from_scratch(&board));

    board.set_piece(c3, None);
    assert_eq!(board.key(), 0);

    // Build custom position with promotion and capture opportunities
    board.set_piece(Square::E1, Some(Piece::new(Color::White, PieceType::King)));
    board.set_piece(Square::E5, Some(Piece::new(Color::Black, PieceType::King)));
    board.set_piece(Square::A4, Some(Piece::new(Color::White, PieceType::Pawn)));
    board.set_piece(Square::B5, Some(Piece::new(Color::Black, PieceType::Rook)));
    assert_eq!(board.key(), recompute_key_from_scratch(&board));

    // No-op move from empty square should not change key
    board.make_move(Move::new(Square::D3, Square::D4, None));
    assert_eq!(board.key(), recompute_key_from_scratch(&board));

    // Capture + promotion: White pawn a4xb5=Q
    board.make_move(Move::new(Square::A4, Square::B5, Some(PieceType::Queen)));
    assert_eq!(board.key(), recompute_key_from_scratch(&board));

    // Black king move: e5-d5
    board.make_move(Move::new(Square::E5, Square::D5, None));
    assert_eq!(board.key(), recompute_key_from_scratch(&board));
}

#[test]
fn test_transposition_produces_identical_key() {
    let mut board1 = Board::gardner_initial();
    board1.make_move(Move::new(Square::B2, Square::B3, None));
    board1.make_move(Move::new(Square::A4, Square::A3, None));
    board1.make_move(Move::new(Square::D2, Square::D3, None));

    let mut board2 = Board::gardner_initial();
    board2.make_move(Move::new(Square::D2, Square::D3, None));
    board2.make_move(Move::new(Square::A4, Square::A3, None));
    board2.make_move(Move::new(Square::B2, Square::B3, None));

    assert_eq!(board1, board2);
    assert_eq!(board1.key(), board2.key());
    assert_eq!(board1.key(), recompute_key_from_scratch(&board1));
}

#[test]
fn test_round_trip_returns_to_initial_key() {
    let initial = Board::gardner_initial();
    let mut board = initial;

    // Knights hop out and back to exact starting position and side to move
    board.make_move(Move::new(Square::B1, Square::A3, None));
    assert_ne!(board.key(), initial.key());
    board.make_move(Move::new(Square::B5, Square::C3, None));
    board.make_move(Move::new(Square::A3, Square::B1, None));
    board.make_move(Move::new(Square::C3, Square::B5, None));

    assert_eq!(board, initial);
    assert_eq!(board.key(), initial.key());
}

fn verify_keys_recursive(board: &Board, depth: u32) {
    assert_eq!(
        board.key(),
        recompute_key_from_scratch(board),
        "Incremental key mismatch at depth {depth}"
    );
    if depth == 0 {
        return;
    }

    for &m in &*generate_legal_moves(board) {
        let mut next = *board;
        next.make_move(m);
        verify_keys_recursive(&next, depth - 1);
    }
}

#[test]
fn test_perft_walk_key_invariant() {
    let board = Board::gardner_initial();
    verify_keys_recursive(&board, 4);
}
