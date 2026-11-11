#![allow(missing_docs)]

use zest::board::Bitboard;
use zest::core::Square;

#[test]
fn test_bitboard_empty_and_all() {
    let empty = Bitboard::empty();
    assert!(empty.is_empty());
    assert_eq!(empty.count(), 0);
    assert_eq!(empty, Bitboard::EMPTY);

    let all = Bitboard::ALL;
    assert!(!all.is_empty());
    assert_eq!(all.count(), 25);

    for &sq in &Square::ALL {
        assert!(!empty.contains(sq));
        assert!(all.contains(sq));
    }
}

#[test]
fn test_bitboard_from_square() {
    for &sq in &Square::ALL {
        let bb = Bitboard::from_square(sq);
        assert_eq!(bb.count(), 1);
        assert!(bb.contains(sq));

        // Other squares should not be contained
        for &other in &Square::ALL {
            if other != sq {
                assert!(!bb.contains(other));
            }
        }
    }
}

#[test]
fn test_bitboard_set_clear_toggle() {
    let mut bb = Bitboard::empty();
    let a1 = Square::A1;
    let c3 = Square::C3;

    // Set A1
    bb.set(a1);
    assert!(bb.contains(a1));
    assert_eq!(bb.count(), 1);

    // Set A1 again (idempotent)
    bb.set(a1);
    assert_eq!(bb.count(), 1);

    // Set C3
    bb.set(c3);
    assert!(bb.contains(c3));
    assert_eq!(bb.count(), 2);

    // Toggle C3 (turns off)
    bb.toggle(c3);
    assert!(!bb.contains(c3));
    assert_eq!(bb.count(), 1);

    // Toggle C3 again (turns back on)
    bb.toggle(c3);
    assert!(bb.contains(c3));
    assert_eq!(bb.count(), 2);

    // Clear A1
    bb.clear(a1);
    assert!(!bb.contains(a1));
    assert_eq!(bb.count(), 1);

    // Clear C3
    bb.clear(c3);
    assert!(bb.is_empty());
    assert_eq!(bb.count(), 0);
}

#[test]
fn test_bitboard_pop_lsb_order() {
    let mut bb = Bitboard::empty();
    assert_eq!(bb.pop_lsb(), None);

    // Put pieces on A1 (idx 0), C3 (idx 12), and E5 (idx 24)
    bb.set(Square::A1);
    bb.set(Square::C3);
    bb.set(Square::E5);
    assert_eq!(bb.count(), 3);

    // pop_lsb should yield in ascending order of index
    assert_eq!(bb.pop_lsb(), Some(Square::A1));
    assert_eq!(bb.count(), 2);

    assert_eq!(bb.pop_lsb(), Some(Square::C3));
    assert_eq!(bb.count(), 1);

    assert_eq!(bb.pop_lsb(), Some(Square::E5));
    assert_eq!(bb.count(), 0);
    assert!(bb.is_empty());

    // Once empty, it returns None
    assert_eq!(bb.pop_lsb(), None);
}

#[test]
fn test_bitboard_pop_lsb_all_squares() {
    let mut bb = Bitboard::ALL;

    // Popping all 25 squares should perfectly match Square::ALL
    for &expected_sq in &Square::ALL {
        assert_eq!(bb.pop_lsb(), Some(expected_sq));
    }

    assert!(bb.is_empty());
    assert_eq!(bb.pop_lsb(), None);
}

#[test]
fn test_bitboard_bitor() {
    let mut bb1 = Bitboard::empty();
    bb1.set(Square::A1);
    bb1.set(Square::C3);

    let mut bb2 = Bitboard::empty();
    bb2.set(Square::C3);
    bb2.set(Square::E5);

    let combined = bb1 | bb2;
    assert_eq!(combined.count(), 3);
    assert!(combined.contains(Square::A1));
    assert!(combined.contains(Square::C3));
    assert!(combined.contains(Square::E5));
    assert!(!combined.contains(Square::B2));
}
