#![allow(missing_docs)]

use std::collections::HashSet;
use zest::core::{Move, ParseMoveError, PieceType, Square};

#[test]
fn test_move_basic() {
    let m = Move::new(Square::A1, Square::A2, None);
    assert_eq!(m.from(), Square::A1);
    assert_eq!(m.to(), Square::A2);
    assert_eq!(m.promotion(), None);
}

#[test]
fn test_move_with_promotion() {
    let m = Move::new(Square::E4, Square::E5, Some(PieceType::Queen));
    assert_eq!(m.from(), Square::E4);
    assert_eq!(m.to(), Square::E5);
    assert_eq!(m.promotion(), Some(PieceType::Queen));
}

#[test]
fn test_move_equality_and_hash() {
    let m1 = Move::new(Square::A2, Square::A3, None);
    let m2 = Move::new(Square::A2, Square::A3, None);
    let m3 = Move::new(Square::A2, Square::A4, None);
    let m4 = Move::new(Square::A4, Square::A5, Some(PieceType::Queen));
    let m5 = Move::new(Square::A4, Square::A5, Some(PieceType::Knight));

    assert_eq!(m1, m2);
    assert_ne!(m1, m3);
    assert_ne!(m4, m5);

    let mut set = HashSet::new();
    set.insert(m1);
    set.insert(m2); // Duplicate
    set.insert(m3);
    set.insert(m4);
    set.insert(m5);

    assert_eq!(set.len(), 4);
    assert!(set.contains(&m1));
    assert!(set.contains(&m3));
    assert!(set.contains(&m4));
    assert!(set.contains(&m5));
}

#[test]
fn test_move_copy() {
    let m1 = Move::new(Square::B1, Square::C3, None);
    let m2 = m1; // Copy

    assert_eq!(m1, m2);
}

#[test]
fn test_move_display() {
    let normal_move = Move::new(Square::A2, Square::A3, None);
    assert_eq!(format!("{normal_move}"), "a2a3");

    let promo_queen = Move::new(Square::E4, Square::E5, Some(PieceType::Queen));
    assert_eq!(format!("{promo_queen}"), "e4e5q");

    let promo_knight = Move::new(Square::C4, Square::C5, Some(PieceType::Knight));
    assert_eq!(format!("{promo_knight}"), "c4c5n");

    let promo_rook = Move::new(Square::B4, Square::B5, Some(PieceType::Rook));
    assert_eq!(format!("{promo_rook}"), "b4b5r");

    let promo_bishop = Move::new(Square::D4, Square::D5, Some(PieceType::Bishop));
    assert_eq!(format!("{promo_bishop}"), "d4d5b");
}

#[test]
fn test_move_from_uci_and_from_str() {
    let normal = Move::from_uci("c2c3").expect("valid UCI move");
    assert_eq!(normal, Move::new(Square::C2, Square::C3, None));
    assert_eq!("c2c3".parse::<Move>(), Ok(normal));

    for (uci, pt) in [
        ("e4e5n", PieceType::Knight),
        ("e4e5b", PieceType::Bishop),
        ("e4e5r", PieceType::Rook),
        ("e4e5q", PieceType::Queen),
        ("e4e5Q", PieceType::Queen),
    ] {
        let m = Move::from_uci(uci).expect("valid promotion UCI move");
        assert_eq!(m, Move::new(Square::E4, Square::E5, Some(pt)));
    }
}

#[test]
fn test_move_from_uci_errors() {
    assert_eq!(Move::from_uci(""), Err(ParseMoveError::InvalidLength));
    assert_eq!(Move::from_uci("a1a"), Err(ParseMoveError::InvalidLength));
    assert_eq!(Move::from_uci("a1a2q1"), Err(ParseMoveError::InvalidLength));
    assert!(Move::from_uci("あa2").is_err());
    assert!(Move::from_uci("あいうえ").is_err());

    assert_eq!(
        Move::from_uci("f1a2"),
        Err(ParseMoveError::InvalidFromSquare)
    );
    assert_eq!(
        Move::from_uci("a6a2"),
        Err(ParseMoveError::InvalidFromSquare)
    );
    assert_eq!(Move::from_uci("a1f2"), Err(ParseMoveError::InvalidToSquare));
    assert_eq!(Move::from_uci("a1a6"), Err(ParseMoveError::InvalidToSquare));

    assert_eq!(Move::from_uci("c3c3"), Err(ParseMoveError::SameSquare));

    assert_eq!(
        Move::from_uci("e4e5k"),
        Err(ParseMoveError::InvalidPromotion)
    );
    assert_eq!(
        Move::from_uci("e4e5p"),
        Err(ParseMoveError::InvalidPromotion)
    );
    assert_eq!(
        Move::from_uci("e4e5x"),
        Err(ParseMoveError::InvalidPromotion)
    );
}
