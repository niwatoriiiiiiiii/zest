#![allow(missing_docs)]

use zest::core::{Color, Square};
use zest::movegen::{king_attacks, knight_attacks, pawn_attacks};

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
