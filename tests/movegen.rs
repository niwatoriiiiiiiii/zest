#![allow(missing_docs)]

use zest::board::Board;
use zest::core::{Color, Move, Piece, PieceType, Square};
use zest::movegen::generate_legal_moves;

#[test]
fn test_initial_position_moves() {
    let board = Board::gardner_initial();
    let moves = generate_legal_moves(&board);

    // In Gardner minichess (5x5), rank 1 is [R, N, B, Q, K].
    // White has only 1 knight (at B1) and 1 queen (at D1).
    // Exactly 7 legal moves exist on the starting turn for White:
    // - 5 pawn pushes: a2a3, b2b3, c2c3, d2d3, e2e3
    // - 2 knight leaps: b1a3, b1c3
    // All other pieces are blocked behind pawns or friendly pieces.
    assert_eq!(moves.len(), 7);

    let expected_moves = [
        Move::new(Square::A2, Square::A3, None),
        Move::new(Square::B2, Square::B3, None),
        Move::new(Square::C2, Square::C3, None),
        Move::new(Square::D2, Square::D3, None),
        Move::new(Square::E2, Square::E3, None),
        Move::new(Square::B1, Square::A3, None),
        Move::new(Square::B1, Square::C3, None),
    ];

    for expected in expected_moves {
        assert!(
            moves.contains(&expected),
            "Expected move {expected} not found in legal moves"
        );
    }
}

#[test]
fn test_pawn_push_blocked() {
    let mut board = Board::empty();
    board.set_piece(Square::A1, Some(Piece::new(Color::White, PieceType::King)));
    board.set_piece(Square::E5, Some(Piece::new(Color::Black, PieceType::King)));

    // White pawn at C2, blocked by black piece at C3
    board.set_piece(Square::C2, Some(Piece::new(Color::White, PieceType::Pawn)));
    board.set_piece(Square::C3, Some(Piece::new(Color::Black, PieceType::Pawn)));

    let moves = generate_legal_moves(&board);

    // Pawn cannot push to C3 (blocked)
    assert!(!moves.contains(&Move::new(Square::C2, Square::C3, None)));
}

#[test]
fn test_pawn_captures() {
    let mut board = Board::empty();
    board.set_piece(Square::A1, Some(Piece::new(Color::White, PieceType::King)));
    board.set_piece(Square::E5, Some(Piece::new(Color::Black, PieceType::King)));

    // White pawn at C2, enemy black pawns at B3 and D3 (no check on White King)
    board.set_piece(Square::C2, Some(Piece::new(Color::White, PieceType::Pawn)));
    board.set_piece(Square::B3, Some(Piece::new(Color::Black, PieceType::Pawn)));
    board.set_piece(Square::D3, Some(Piece::new(Color::Black, PieceType::Pawn)));

    let moves = generate_legal_moves(&board);

    // Can push forward to empty C3
    assert!(moves.contains(&Move::new(Square::C2, Square::C3, None)));
    // Can capture diagonally at B3 and D3
    assert!(moves.contains(&Move::new(Square::C2, Square::B3, None)));
    assert!(moves.contains(&Move::new(Square::C2, Square::D3, None)));
}

#[test]
fn test_pawn_promotion() {
    let mut board = Board::empty();
    board.set_piece(Square::A1, Some(Piece::new(Color::White, PieceType::King)));
    board.set_piece(Square::E1, Some(Piece::new(Color::Black, PieceType::King)));

    // White pawn at C4 (one step from promotion rank 5)
    board.set_piece(Square::C4, Some(Piece::new(Color::White, PieceType::Pawn)));
    // Black pawn at D5 (diagonal capture promotion target)
    board.set_piece(Square::D5, Some(Piece::new(Color::Black, PieceType::Pawn)));

    let moves = generate_legal_moves(&board);

    // Straight push promotions (C4 -> C5)
    assert!(moves.contains(&Move::new(Square::C4, Square::C5, Some(PieceType::Queen))));
    assert!(moves.contains(&Move::new(Square::C4, Square::C5, Some(PieceType::Rook))));
    assert!(moves.contains(&Move::new(Square::C4, Square::C5, Some(PieceType::Bishop))));
    assert!(moves.contains(&Move::new(Square::C4, Square::C5, Some(PieceType::Knight))));
    assert!(!moves.contains(&Move::new(Square::C4, Square::C5, None)));

    // Diagonal capture promotions (C4 -> D5)
    assert!(moves.contains(&Move::new(Square::C4, Square::D5, Some(PieceType::Queen))));
    assert!(moves.contains(&Move::new(Square::C4, Square::D5, Some(PieceType::Rook))));
    assert!(moves.contains(&Move::new(Square::C4, Square::D5, Some(PieceType::Bishop))));
    assert!(moves.contains(&Move::new(Square::C4, Square::D5, Some(PieceType::Knight))));
}

#[test]
fn test_must_escape_check() {
    let mut board = Board::empty();
    // White King at C1
    board.set_piece(Square::C1, Some(Piece::new(Color::White, PieceType::King)));
    // Black King at A5
    board.set_piece(Square::A5, Some(Piece::new(Color::Black, PieceType::King)));
    // Black Rook at C5 gives check along the C-file
    board.set_piece(Square::C5, Some(Piece::new(Color::Black, PieceType::Rook)));

    // Unrelated White pawn at E2
    board.set_piece(Square::E2, Some(Piece::new(Color::White, PieceType::Pawn)));

    let moves = generate_legal_moves(&board);

    // E2 pawn cannot move because king is in check and pawn cannot resolve it
    assert!(!moves.contains(&Move::new(Square::E2, Square::E3, None)));

    // King cannot remain on the C-file (cannot move to C2)
    assert!(!moves.contains(&Move::new(Square::C1, Square::C2, None)));

    // King CAN move out of the check (to B1, D1, B2, D2)
    assert!(moves.contains(&Move::new(Square::C1, Square::B1, None)));
    assert!(moves.contains(&Move::new(Square::C1, Square::D1, None)));
}

#[test]
fn test_pinned_piece_cannot_move() {
    let mut board = Board::empty();
    // White King at C1
    board.set_piece(Square::C1, Some(Piece::new(Color::White, PieceType::King)));
    // Black King at A5
    board.set_piece(Square::A5, Some(Piece::new(Color::Black, PieceType::King)));

    // White Bishop at C3 is pinned to King at C1 by Black Rook at C5
    board.set_piece(
        Square::C3,
        Some(Piece::new(Color::White, PieceType::Bishop)),
    );
    board.set_piece(Square::C5, Some(Piece::new(Color::Black, PieceType::Rook)));

    let moves = generate_legal_moves(&board);

    // Bishop cannot move diagonally (e.g. to B4, D4, B2, D2) because it exposes king to check
    assert!(!moves.contains(&Move::new(Square::C3, Square::B4, None)));
    assert!(!moves.contains(&Move::new(Square::C3, Square::D4, None)));
    assert!(!moves.contains(&Move::new(Square::C3, Square::B2, None)));
    assert!(!moves.contains(&Move::new(Square::C3, Square::D2, None)));
}

#[test]
fn test_checkmate_has_no_legal_moves() {
    let mut board = Board::empty();
    // White King trapped in corner A1
    board.set_piece(Square::A1, Some(Piece::new(Color::White, PieceType::King)));
    // Black Rook at A5 checks on A-file
    board.set_piece(Square::A5, Some(Piece::new(Color::Black, PieceType::Rook)));
    // Black Queen at B2 covers B-file and escape squares
    board.set_piece(Square::B2, Some(Piece::new(Color::Black, PieceType::Queen)));
    // Black King at C3 protects the Queen at B2 (so White King cannot capture B2!)
    board.set_piece(Square::C3, Some(Piece::new(Color::Black, PieceType::King)));

    let moves = generate_legal_moves(&board);

    // White is checkmated: exactly 0 legal moves
    assert!(moves.is_empty());
}

#[test]
fn test_king_cannot_retreat_along_xray_ray() {
    let mut board = Board::empty();
    // White King at C2 checked by Black Rook at C5 along C-file
    board.set_piece(Square::C2, Some(Piece::new(Color::White, PieceType::King)));
    board.set_piece(Square::C5, Some(Piece::new(Color::Black, PieceType::Rook)));
    board.set_piece(Square::A5, Some(Piece::new(Color::Black, PieceType::King)));

    let moves = generate_legal_moves(&board);

    // Retreating to C1 is illegal because the king itself leaves C2 (X-ray check)
    assert!(!moves.contains(&Move::new(Square::C2, Square::C1, None)));
    assert!(!moves.contains(&Move::new(Square::C2, Square::C3, None)));

    // Stepping off the C-file is legal
    assert!(moves.contains(&Move::new(Square::C2, Square::B1, None)));
    assert!(moves.contains(&Move::new(Square::C2, Square::D1, None)));
}

#[test]
fn test_king_can_capture_unprotected_checker() {
    let mut board = Board::empty();
    board.set_piece(Square::C2, Some(Piece::new(Color::White, PieceType::King)));
    board.set_piece(Square::C3, Some(Piece::new(Color::Black, PieceType::Queen)));
    board.set_piece(Square::A5, Some(Piece::new(Color::Black, PieceType::King)));

    let moves = generate_legal_moves(&board);

    assert!(moves.contains(&Move::new(Square::C2, Square::C3, None)));
    assert!(moves.contains(&Move::new(Square::C2, Square::B1, None)));
    assert!(moves.contains(&Move::new(Square::C2, Square::D1, None)));
    assert!(!moves.contains(&Move::new(Square::C2, Square::C1, None)));
}
