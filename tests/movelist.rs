#![allow(missing_docs)]

use zest::core::{Move, PieceType, Square};
use zest::movegen::MoveList;

#[test]
fn test_movelist_new_and_default() {
    let list_new = MoveList::new();
    assert_eq!(list_new.len(), 0);
    assert!(list_new.is_empty());
    assert_eq!(list_new.as_slice(), &[]);

    let list_default = MoveList::default();
    assert_eq!(list_default.len(), 0);
    assert!(list_default.is_empty());
    assert_eq!(list_new, list_default);
}

#[test]
fn test_movelist_push_and_len() {
    let mut list = MoveList::new();
    let m1 = Move::new(Square::A1, Square::A2, None);
    let m2 = Move::new(Square::B1, Square::C3, None);
    let m3 = Move::new(Square::C4, Square::C5, Some(PieceType::Queen));

    list.push(m1);
    assert_eq!(list.len(), 1);
    assert!(!list.is_empty());
    assert_eq!(list[0], m1);

    list.push(m2);
    assert_eq!(list.len(), 2);
    assert_eq!(list[1], m2);

    list.push(m3);
    assert_eq!(list.len(), 3);
    assert_eq!(list[2], m3);

    assert_eq!(list.as_slice(), &[m1, m2, m3]);
}

#[test]
fn test_movelist_clear() {
    let mut list = MoveList::new();
    list.push(Move::new(Square::A1, Square::A2, None));
    list.push(Move::new(Square::B1, Square::C3, None));
    assert_eq!(list.len(), 2);

    list.clear();
    assert_eq!(list.len(), 0);
    assert!(list.is_empty());
    assert_eq!(list.as_slice(), &[]);

    // Can push again after clearing
    let m = Move::new(Square::D1, Square::D5, None);
    list.push(m);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0], m);
}

#[test]
fn test_movelist_deref_and_mutation() {
    let mut list = MoveList::new();
    let m1 = Move::new(Square::A1, Square::A2, None);
    let m2 = Move::new(Square::B1, Square::B2, None);
    list.push(m1);
    list.push(m2);

    // Deref: iteration via .iter()
    let mut collected = Vec::new();
    for &m in list.iter() {
        collected.push(m);
    }
    assert_eq!(collected, vec![m1, m2]);

    // DerefMut: mutate element in place
    let m_new = Move::new(Square::E1, Square::E2, None);
    list[0] = m_new;
    assert_eq!(list[0], m_new);
    assert_eq!(list[1], m2);
}

#[test]
fn test_movelist_retain() {
    let mut list = MoveList::new();
    let m1 = Move::new(Square::A1, Square::A2, None);
    let m2 = Move::new(Square::B1, Square::C3, None);
    let m3 = Move::new(Square::A2, Square::A3, None);
    let m4 = Move::new(Square::C1, Square::E3, None);

    list.push(m1);
    list.push(m2);
    list.push(m3);
    list.push(m4);

    // Retain only moves from file A
    list.retain(|m| m.from().file() == 0);

    assert_eq!(list.len(), 2);
    assert_eq!(list.as_slice(), &[m1, m3]);

    // Retain none
    list.retain(|_| false);
    assert_eq!(list.len(), 0);
    assert!(list.is_empty());
}

#[test]
fn test_movelist_max_capacity() {
    let mut list = MoveList::new();

    for i in 0_u8..128_u8 {
        let from = Square::from_index(i % 25).unwrap();
        let to = Square::from_index((i + 1) % 25).unwrap();
        list.push(Move::new(from, to, None));
    }

    assert_eq!(list.len(), 128);
    assert!(!list.is_empty());
    assert_eq!(list.as_slice().len(), 128);
}

#[test]
fn test_movelist_clone_copy_debug_eq() {
    let mut list = MoveList::new();
    list.push(Move::new(Square::A1, Square::A2, None));

    let copy = list;
    assert_eq!(list, copy);

    let debug_str = format!("{list:?}");
    assert!(debug_str.contains("MoveList"));
}
