#![allow(missing_docs)]

use zest::board::{Bitboard, Board};
use zest::core::{Color, Piece, PieceType, Square};
use zest::movegen::{
    bishop_attacks, is_square_attacked, king_attacks, knight_attacks, pawn_attacks, queen_attacks,
    rook_attacks,
};

#[test]
fn test_knight_attacks_center() {
    // Knight at C3 (center of 5x5 board) has all 8 moves on board
    let attacks = knight_attacks(Square::C3);
    assert_eq!(attacks.count(), 8);

    assert!(attacks.contains(Square::B1));
    assert!(attacks.contains(Square::D1));
    assert!(attacks.contains(Square::A2));
    assert!(attacks.contains(Square::E2));
    assert!(attacks.contains(Square::A4));
    assert!(attacks.contains(Square::E4));
    assert!(attacks.contains(Square::B5));
    assert!(attacks.contains(Square::D5));
}

#[test]
fn test_knight_attacks_corner() {
    // Knight at A1 (corner) can only reach B3 and C2
    let attacks = knight_attacks(Square::A1);
    assert_eq!(attacks.count(), 2);
    assert!(attacks.contains(Square::B3));
    assert!(attacks.contains(Square::C2));
}

#[test]
fn test_knight_attacks_edge() {
    // Knight at A3 (edge) can reach B1, C2, C4, B5
    let attacks = knight_attacks(Square::A3);
    assert_eq!(attacks.count(), 4);
    assert!(attacks.contains(Square::B1));
    assert!(attacks.contains(Square::C2));
    assert!(attacks.contains(Square::C4));
    assert!(attacks.contains(Square::B5));
}

#[test]
fn test_king_attacks_center() {
    // King at C3 (center) can reach all 8 adjacent squares
    let attacks = king_attacks(Square::C3);
    assert_eq!(attacks.count(), 8);

    assert!(attacks.contains(Square::B2));
    assert!(attacks.contains(Square::C2));
    assert!(attacks.contains(Square::D2));
    assert!(attacks.contains(Square::B3));
    assert!(attacks.contains(Square::D3));
    assert!(attacks.contains(Square::B4));
    assert!(attacks.contains(Square::C4));
    assert!(attacks.contains(Square::D4));
}

#[test]
fn test_king_attacks_corner() {
    // King at A1 (corner) can only reach A2, B1, B2 (3 squares)
    let attacks = king_attacks(Square::A1);
    assert_eq!(attacks.count(), 3);
    assert!(attacks.contains(Square::A2));
    assert!(attacks.contains(Square::B1));
    assert!(attacks.contains(Square::B2));
}

#[test]
fn test_king_attacks_edge() {
    // King at A3 (edge) can reach A2, B2, B3, B4, A4 (5 squares)
    let attacks = king_attacks(Square::A3);
    assert_eq!(attacks.count(), 5);
    assert!(attacks.contains(Square::A2));
    assert!(attacks.contains(Square::B2));
    assert!(attacks.contains(Square::B3));
    assert!(attacks.contains(Square::B4));
    assert!(attacks.contains(Square::A4));
}

#[test]
fn test_pawn_attacks_white() {
    // White pawn at C2 attacks B3 and D3 (diagonal up)
    let attacks = pawn_attacks(Square::C2, Color::White);
    assert_eq!(attacks.count(), 2);
    assert!(attacks.contains(Square::B3));
    assert!(attacks.contains(Square::D3));

    // White pawn at A2 (left edge) only attacks B3 (right diagonal up)
    let left_edge = pawn_attacks(Square::A2, Color::White);
    assert_eq!(left_edge.count(), 1);
    assert!(left_edge.contains(Square::B3));

    // White pawn at E2 (right edge) only attacks D3 (left diagonal up)
    let right_edge = pawn_attacks(Square::E2, Color::White);
    assert_eq!(right_edge.count(), 1);
    assert!(right_edge.contains(Square::D3));
}

#[test]
fn test_pawn_attacks_black() {
    // Black pawn at C4 attacks B3 and D3 (diagonal down)
    let attacks = pawn_attacks(Square::C4, Color::Black);
    assert_eq!(attacks.count(), 2);
    assert!(attacks.contains(Square::B3));
    assert!(attacks.contains(Square::D3));

    // Black pawn at A4 (left edge) only attacks B3 (right diagonal down)
    let left_edge = pawn_attacks(Square::A4, Color::Black);
    assert_eq!(left_edge.count(), 1);
    assert!(left_edge.contains(Square::B3));

    // Black pawn at E4 (right edge) only attacks D3 (left diagonal down)
    let right_edge = pawn_attacks(Square::E4, Color::Black);
    assert_eq!(right_edge.count(), 1);
    assert!(right_edge.contains(Square::D3));
}

#[test]
fn test_rook_attacks_open_board() {
    // Rook at C3 on open board: 4 horizontal + 4 vertical = 8 squares
    let attacks = rook_attacks(Square::C3, Bitboard::empty());
    assert_eq!(attacks.count(), 8);

    assert!(attacks.contains(Square::C1));
    assert!(attacks.contains(Square::C2));
    assert!(attacks.contains(Square::C4));
    assert!(attacks.contains(Square::C5));
    assert!(attacks.contains(Square::A3));
    assert!(attacks.contains(Square::B3));
    assert!(attacks.contains(Square::D3));
    assert!(attacks.contains(Square::E3));

    // Rook at A1 on open board: 4 vertical (A2..A5) + 4 horizontal (B1..E1) = 8 squares
    let corner = rook_attacks(Square::A1, Bitboard::empty());
    assert_eq!(corner.count(), 8);
    assert!(corner.contains(Square::A2));
    assert!(corner.contains(Square::A3));
    assert!(corner.contains(Square::A4));
    assert!(corner.contains(Square::A5));
    assert!(corner.contains(Square::B1));
    assert!(corner.contains(Square::C1));
    assert!(corner.contains(Square::D1));
    assert!(corner.contains(Square::E1));
}

#[test]
fn test_rook_attacks_blocked() {
    // Rook at C3 with obstacle at C4
    let mut occupied = Bitboard::empty();
    occupied.set(Square::C4);

    let attacks = rook_attacks(Square::C3, occupied);
    // Should hit C4, but NOT C5 (blocked)
    assert!(attacks.contains(Square::C4));
    assert!(!attacks.contains(Square::C5));
    // Other 3 directions should be intact
    assert_eq!(attacks.count(), 7);

    // Completely surrounded by adjacent pieces
    let mut surrounded = Bitboard::empty();
    surrounded.set(Square::C4);
    surrounded.set(Square::C2);
    surrounded.set(Square::B3);
    surrounded.set(Square::D3);

    let trapped = rook_attacks(Square::C3, surrounded);
    assert_eq!(trapped.count(), 4);
    assert!(trapped.contains(Square::C4));
    assert!(trapped.contains(Square::C2));
    assert!(trapped.contains(Square::B3));
    assert!(trapped.contains(Square::D3));
}

#[test]
fn test_bishop_attacks_open_board() {
    // Bishop at C3 on open board: 2 squares in each of 4 diagonal rays = 8 squares
    let attacks = bishop_attacks(Square::C3, Bitboard::empty());
    assert_eq!(attacks.count(), 8);

    assert!(attacks.contains(Square::A1));
    assert!(attacks.contains(Square::B2));
    assert!(attacks.contains(Square::D4));
    assert!(attacks.contains(Square::E5));

    assert!(attacks.contains(Square::A5));
    assert!(attacks.contains(Square::B4));
    assert!(attacks.contains(Square::D2));
    assert!(attacks.contains(Square::E1));

    // Bishop at A1 (corner): 4 squares along the long diagonal
    let corner = bishop_attacks(Square::A1, Bitboard::empty());
    assert_eq!(corner.count(), 4);
    assert!(corner.contains(Square::B2));
    assert!(corner.contains(Square::C3));
    assert!(corner.contains(Square::D4));
    assert!(corner.contains(Square::E5));
}

#[test]
fn test_bishop_attacks_blocked() {
    // Bishop at C3 with obstacle at D4
    let mut occupied = Bitboard::empty();
    occupied.set(Square::D4);

    let attacks = bishop_attacks(Square::C3, occupied);
    // Should reach D4, but NOT E5
    assert!(attacks.contains(Square::D4));
    assert!(!attacks.contains(Square::E5));
    // 8 - 1 = 7 squares
    assert_eq!(attacks.count(), 7);
}

#[test]
fn test_queen_attacks_open_board() {
    // Queen at C3: rook (8) + bishop (8) = 16 squares
    let attacks = queen_attacks(Square::C3, Bitboard::empty());
    assert_eq!(attacks.count(), 16);

    // Queen at A1: rook (8) + bishop (4) = 12 squares
    let corner = queen_attacks(Square::A1, Bitboard::empty());
    assert_eq!(corner.count(), 12);
}

#[test]
fn test_queen_attacks_blocked() {
    // Queen at C3 with blockers at C4 (orthogonal) and D4 (diagonal)
    let mut occupied = Bitboard::empty();
    occupied.set(Square::C4);
    occupied.set(Square::D4);

    let attacks = queen_attacks(Square::C3, occupied);
    // Blocked rays stop behind blockers
    assert!(attacks.contains(Square::C4));
    assert!(!attacks.contains(Square::C5));
    assert!(attacks.contains(Square::D4));
    assert!(!attacks.contains(Square::E5));

    // 16 - 2 = 14 squares
    assert_eq!(attacks.count(), 14);
}

#[test]
fn test_is_square_attacked_pawn() {
    let mut board = Board::empty();
    // White pawn at C2
    board.set_piece(Square::C2, Some(Piece::new(Color::White, PieceType::Pawn)));

    // Diagonals B3 and D3 are attacked by White
    assert!(is_square_attacked(&board, Square::B3, Color::White));
    assert!(is_square_attacked(&board, Square::D3, Color::White));

    // Straight forward C3 is NOT an attack
    assert!(!is_square_attacked(&board, Square::C3, Color::White));

    // Backward diagonals B1, D1 are NOT attacks
    assert!(!is_square_attacked(&board, Square::B1, Color::White));
    assert!(!is_square_attacked(&board, Square::D1, Color::White));

    // Black pawn at C4 attacks B3 and D3 (downwards)
    board.set_piece(Square::C4, Some(Piece::new(Color::Black, PieceType::Pawn)));
    assert!(is_square_attacked(&board, Square::B3, Color::Black));
    assert!(is_square_attacked(&board, Square::D3, Color::Black));
    assert!(!is_square_attacked(&board, Square::C3, Color::Black));
}

#[test]
fn test_is_square_attacked_knight() {
    let mut board = Board::empty();
    board.set_piece(
        Square::C3,
        Some(Piece::new(Color::White, PieceType::Knight)),
    );

    // All 8 knight squares should be attacked
    assert!(is_square_attacked(&board, Square::B1, Color::White));
    assert!(is_square_attacked(&board, Square::D1, Color::White));
    assert!(is_square_attacked(&board, Square::A2, Color::White));
    assert!(is_square_attacked(&board, Square::E2, Color::White));
    assert!(is_square_attacked(&board, Square::A4, Color::White));
    assert!(is_square_attacked(&board, Square::E4, Color::White));
    assert!(is_square_attacked(&board, Square::B5, Color::White));
    assert!(is_square_attacked(&board, Square::D5, Color::White));

    // Other squares should NOT be attacked
    assert!(!is_square_attacked(&board, Square::C2, Color::White));
    assert!(!is_square_attacked(&board, Square::C4, Color::White));
    assert!(!is_square_attacked(&board, Square::B3, Color::White));
}

#[test]
fn test_is_square_attacked_king() {
    let mut board = Board::empty();
    board.set_piece(Square::C3, Some(Piece::new(Color::Black, PieceType::King)));

    // Surrounding 8 squares are attacked by Black
    assert!(is_square_attacked(&board, Square::B2, Color::Black));
    assert!(is_square_attacked(&board, Square::C2, Color::Black));
    assert!(is_square_attacked(&board, Square::D2, Color::Black));
    assert!(is_square_attacked(&board, Square::B3, Color::Black));
    assert!(is_square_attacked(&board, Square::D3, Color::Black));
    assert!(is_square_attacked(&board, Square::B4, Color::Black));
    assert!(is_square_attacked(&board, Square::C4, Color::Black));
    assert!(is_square_attacked(&board, Square::D4, Color::Black));

    // Distance 2 squares are NOT attacked
    assert!(!is_square_attacked(&board, Square::A1, Color::Black));
    assert!(!is_square_attacked(&board, Square::C1, Color::Black));
}

#[test]
fn test_is_square_attacked_sliding_and_blockers() {
    let mut board = Board::empty();
    // White Rook on C1
    board.set_piece(Square::C1, Some(Piece::new(Color::White, PieceType::Rook)));

    // Open ray reaches C2..C5
    assert!(is_square_attacked(&board, Square::C2, Color::White));
    assert!(is_square_attacked(&board, Square::C3, Color::White));
    assert!(is_square_attacked(&board, Square::C4, Color::White));
    assert!(is_square_attacked(&board, Square::C5, Color::White));

    // Put obstacle on C3
    board.set_piece(Square::C3, Some(Piece::new(Color::Black, PieceType::Pawn)));

    // C2 is open, C3 is the obstacle itself (attacked!)
    assert!(is_square_attacked(&board, Square::C2, Color::White));
    assert!(is_square_attacked(&board, Square::C3, Color::White));

    // C4 and C5 are blocked behind C3
    assert!(!is_square_attacked(&board, Square::C4, Color::White));
    assert!(!is_square_attacked(&board, Square::C5, Color::White));

    // White Bishop on A1
    board.set_piece(
        Square::A1,
        Some(Piece::new(Color::White, PieceType::Bishop)),
    );
    // Open ray reaches B2, C3 (now blocked)
    assert!(is_square_attacked(&board, Square::B2, Color::White));
    assert!(is_square_attacked(&board, Square::C3, Color::White));
    // D4 and E5 are blocked behind C3
    assert!(!is_square_attacked(&board, Square::D4, Color::White));
    assert!(!is_square_attacked(&board, Square::E5, Color::White));

    // White Queen on E3
    board.set_piece(Square::E3, Some(Piece::new(Color::White, PieceType::Queen)));
    // Queen attacks orthogonal and diagonal
    assert!(is_square_attacked(&board, Square::D3, Color::White));
    assert!(is_square_attacked(&board, Square::E4, Color::White));
    assert!(is_square_attacked(&board, Square::D4, Color::White));
    assert!(is_square_attacked(&board, Square::D2, Color::White));
}

#[test]
fn test_is_square_attacked_gardner_initial() {
    let board = Board::gardner_initial();

    // Neither king should be in check in the starting position
    assert!(!is_square_attacked(&board, Square::E1, Color::Black));
    assert!(!is_square_attacked(&board, Square::E5, Color::White));

    // Center square C3 is attacked by White knights (B1, D1) and Black knights (B5, D5)
    assert!(is_square_attacked(&board, Square::C3, Color::White));
    assert!(is_square_attacked(&board, Square::C3, Color::Black));

    // A3 is attacked by knight at B1 (White) and B5 (Black)
    assert!(is_square_attacked(&board, Square::A3, Color::White));
    assert!(is_square_attacked(&board, Square::A3, Color::Black));

    // Pawns on rank 2 protect each other diagonally on rank 3
    // E.g. White pawn at B2 attacks A3 and C3
    assert!(is_square_attacked(&board, Square::A3, Color::White));
    assert!(is_square_attacked(&board, Square::C3, Color::White));
}
