#![allow(missing_docs)]

use zest::core::{Color, ParseSquareError, Piece, PieceType, Square};

#[test]
fn test_color_opposite() {
    assert_eq!(Color::White.opposite(), Color::Black);
    assert_eq!(!Color::White, Color::Black);
    assert_eq!(!Color::Black, Color::White);
}

#[test]
fn test_piece_type_char() {
    assert_eq!(PieceType::Pawn.to_char(), 'p');
    assert_eq!(PieceType::Knight.to_char(), 'n');
    assert_eq!(PieceType::Bishop.to_char(), 'b');
    assert_eq!(PieceType::Rook.to_char(), 'r');
    assert_eq!(PieceType::Queen.to_char(), 'q');
    assert_eq!(PieceType::King.to_char(), 'k');
}

#[test]
fn test_square_valid() {
    let a1 = Square::new(0, 0).unwrap();
    assert_eq!(a1, Square::A1);
    assert_eq!(a1.file(), 0);
    assert_eq!(a1.rank(), 0);
    assert_eq!(a1.index(), 0);

    let e5 = Square::new(4, 4).unwrap();
    assert_eq!(e5, Square::E5);
    assert_eq!(e5.file(), 4);
    assert_eq!(e5.rank(), 4);
    assert_eq!(e5.index(), 24);

    assert_eq!(Square::ALL.len(), 25);
    for (i, sq) in Square::ALL.iter().enumerate() {
        assert_eq!(sq.index(), i);
    }
}

#[test]
fn test_square_out_of_bounds() {
    assert_eq!(Square::new(5, 0), None);
    assert_eq!(Square::new(0, 5), None);
    assert_eq!(Square::new(5, 5), None);
}

#[test]
fn test_all_squares_consistency() {
    for (i, &sq) in Square::ALL.iter().enumerate() {
        assert_eq!(sq.index(), i);
        assert_eq!(sq.as_u8() as usize, i);
        assert_eq!(Square::new(sq.file(), sq.rank()), Some(sq));
    }
}

#[test]
fn test_piece_char() {
    let white_pawn = Piece::new(Color::White, PieceType::Pawn);
    let black_knight = Piece::new(Color::Black, PieceType::Knight);

    assert_eq!(white_pawn.to_char(), 'P');
    assert_eq!(black_knight.to_char(), 'n');
    assert_eq!(white_pawn.color, Color::White);
    assert_eq!(white_pawn.piece_type, PieceType::Pawn);
}

#[test]
fn test_square_display() {
    assert_eq!(format!("{}", Square::A1), "a1");
    assert_eq!(format!("{}", Square::C3), "c3");
    assert_eq!(format!("{}", Square::E5), "e5");

    let expected_names = [
        "a1", "b1", "c1", "d1", "e1", "a2", "b2", "c2", "d2", "e2", "a3", "b3", "c3", "d3", "e3",
        "a4", "b4", "c4", "d4", "e4", "a5", "b5", "c5", "d5", "e5",
    ];

    for (i, &sq) in Square::ALL.iter().enumerate() {
        assert_eq!(format!("{sq}"), expected_names[i]);
    }
}

#[test]
fn test_piece_type_from_char() {
    let pairs = [
        ('p', PieceType::Pawn),
        ('P', PieceType::Pawn),
        ('n', PieceType::Knight),
        ('N', PieceType::Knight),
        ('b', PieceType::Bishop),
        ('B', PieceType::Bishop),
        ('r', PieceType::Rook),
        ('R', PieceType::Rook),
        ('q', PieceType::Queen),
        ('Q', PieceType::Queen),
        ('k', PieceType::King),
        ('K', PieceType::King),
    ];

    for (ch, expected) in pairs {
        assert_eq!(PieceType::from_char(ch), Some(expected));
    }

    assert_eq!(PieceType::from_char('x'), None);
    assert_eq!(PieceType::from_char('1'), None);
}

#[test]
fn test_piece_from_char_roundtrip() {
    for color in [Color::White, Color::Black] {
        for piece_type in [
            PieceType::Pawn,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Rook,
            PieceType::Queen,
            PieceType::King,
        ] {
            let piece = Piece::new(color, piece_type);
            assert_eq!(Piece::from_char(piece.to_char()), Some(piece));
        }
    }

    assert_eq!(Piece::from_char('x'), None);
    assert_eq!(Piece::from_char(' '), None);
}

#[test]
fn test_square_from_str_and_ascii() {
    for &sq in &Square::ALL {
        let s = format!("{sq}");
        assert_eq!(s.parse::<Square>(), Ok(sq));
        assert_eq!(Square::from_ascii(s.as_bytes()), Some(sq));
    }

    for invalid in ["", "a", "a11", "f1", "a6", "a0", "A1", "e9", "あ1"] {
        assert_eq!(invalid.parse::<Square>(), Err(ParseSquareError));
    }
}
