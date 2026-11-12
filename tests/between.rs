#![allow(missing_docs)]

use zest::board::Bitboard;
use zest::core::Square;
use zest::movegen::between;

#[test]
fn test_between_horizontal() {
    let a1 = Square::A1;
    let e1 = Square::E1;
    let b1 = Square::B1;
    let c1 = Square::C1;
    let d1 = Square::D1;

    let mut expected = Bitboard::empty();
    expected.set(b1);
    expected.set(c1);
    expected.set(d1);

    assert_eq!(between(a1, e1), expected);
    assert_eq!(between(e1, a1), expected);

    // Adjacent squares have no squares between them
    assert_eq!(between(a1, b1), Bitboard::EMPTY);
}

#[test]
fn test_between_vertical() {
    let a1 = Square::A1;
    let a5 = Square::A5;
    let a2 = Square::A2;
    let a3 = Square::A3;
    let a4 = Square::A4;

    let mut expected = Bitboard::empty();
    expected.set(a2);
    expected.set(a3);
    expected.set(a4);

    assert_eq!(between(a1, a5), expected);
    assert_eq!(between(a5, a1), expected);

    assert_eq!(between(a1, a2), Bitboard::EMPTY);
}

#[test]
fn test_between_diagonal() {
    let a1 = Square::A1;
    let e5 = Square::E5;
    let b2 = Square::B2;
    let c3 = Square::C3;
    let d4 = Square::D4;

    let mut expected = Bitboard::empty();
    expected.set(b2);
    expected.set(c3);
    expected.set(d4);

    assert_eq!(between(a1, e5), expected);
    assert_eq!(between(e5, a1), expected);

    // Anti-diagonal: A5 to E1
    let a5 = Square::A5;
    let e1 = Square::E1;
    let b4 = Square::B4;
    let d2 = Square::D2;

    let mut expected_anti = Bitboard::empty();
    expected_anti.set(b4);
    expected_anti.set(c3);
    expected_anti.set(d2);

    assert_eq!(between(a5, e1), expected_anti);
    assert_eq!(between(e1, a5), expected_anti);
}

#[test]
fn test_between_not_on_line() {
    // Knight move target
    assert_eq!(between(Square::A1, Square::B3), Bitboard::EMPTY);
    assert_eq!(between(Square::B3, Square::A1), Bitboard::EMPTY);

    // Non-collinear squares
    assert_eq!(between(Square::A1, Square::D2), Bitboard::EMPTY);

    // Same square
    assert_eq!(between(Square::C3, Square::C3), Bitboard::EMPTY);
}
