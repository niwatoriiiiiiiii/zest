#![allow(missing_docs)]

use std::collections::HashSet;
use zest::core::{Move, PieceType, Square};

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
