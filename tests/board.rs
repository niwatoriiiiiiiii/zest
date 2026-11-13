#![allow(missing_docs)]

use zest::board::Board;
use zest::core::{Color, Move, Piece, PieceType, Square};

#[test]
fn test_empty_board() {
    let board = Board::empty();
    assert_eq!(board.side_to_move(), Color::White);

    for &sq in &Square::ALL {
        assert_eq!(
            board.piece_at(sq),
            None,
            "Expected empty square at index {}",
            sq.index()
        );
    }
}

#[test]
fn test_gardner_initial_board() {
    let board = Board::gardner_initial();
    assert_eq!(board.side_to_move(), Color::White);

    // Rank 1: White back rank (R, N, B, Q, K)
    assert_eq!(
        board.piece_at(Square::A1),
        Some(Piece::new(Color::White, PieceType::Rook))
    );
    assert_eq!(
        board.piece_at(Square::B1),
        Some(Piece::new(Color::White, PieceType::Knight))
    );
    assert_eq!(
        board.piece_at(Square::C1),
        Some(Piece::new(Color::White, PieceType::Bishop))
    );
    assert_eq!(
        board.piece_at(Square::D1),
        Some(Piece::new(Color::White, PieceType::Queen))
    );
    assert_eq!(
        board.piece_at(Square::E1),
        Some(Piece::new(Color::White, PieceType::King))
    );

    // Rank 2: White pawns
    let white_pawn = Some(Piece::new(Color::White, PieceType::Pawn));
    assert_eq!(board.piece_at(Square::A2), white_pawn);
    assert_eq!(board.piece_at(Square::B2), white_pawn);
    assert_eq!(board.piece_at(Square::C2), white_pawn);
    assert_eq!(board.piece_at(Square::D2), white_pawn);
    assert_eq!(board.piece_at(Square::E2), white_pawn);

    // Rank 3: Empty squares
    assert_eq!(board.piece_at(Square::A3), None);
    assert_eq!(board.piece_at(Square::B3), None);
    assert_eq!(board.piece_at(Square::C3), None);
    assert_eq!(board.piece_at(Square::D3), None);
    assert_eq!(board.piece_at(Square::E3), None);

    // Rank 4: Black pawns
    let black_pawn = Some(Piece::new(Color::Black, PieceType::Pawn));
    assert_eq!(board.piece_at(Square::A4), black_pawn);
    assert_eq!(board.piece_at(Square::B4), black_pawn);
    assert_eq!(board.piece_at(Square::C4), black_pawn);
    assert_eq!(board.piece_at(Square::D4), black_pawn);
    assert_eq!(board.piece_at(Square::E4), black_pawn);

    // Rank 5: Black back rank (R, N, B, Q, K)
    assert_eq!(
        board.piece_at(Square::A5),
        Some(Piece::new(Color::Black, PieceType::Rook))
    );
    assert_eq!(
        board.piece_at(Square::B5),
        Some(Piece::new(Color::Black, PieceType::Knight))
    );
    assert_eq!(
        board.piece_at(Square::C5),
        Some(Piece::new(Color::Black, PieceType::Bishop))
    );
    assert_eq!(
        board.piece_at(Square::D5),
        Some(Piece::new(Color::Black, PieceType::Queen))
    );
    assert_eq!(
        board.piece_at(Square::E5),
        Some(Piece::new(Color::Black, PieceType::King))
    );
}

#[test]
fn test_set_piece_and_clear() {
    let mut board = Board::empty();
    let c3 = Square::C3;

    assert_eq!(board.piece_at(c3), None);

    // Place a white queen on C3
    let white_queen = Piece::new(Color::White, PieceType::Queen);
    board.set_piece(c3, Some(white_queen));
    assert_eq!(board.piece_at(c3), Some(white_queen));

    // Overwrite with a black knight
    let black_knight = Piece::new(Color::Black, PieceType::Knight);
    board.set_piece(c3, Some(black_knight));
    assert_eq!(board.piece_at(c3), Some(black_knight));

    // Clear the square
    board.set_piece(c3, None);
    assert_eq!(board.piece_at(c3), None);
}

#[test]
fn test_board_equality_and_copy() {
    let board1 = Board::gardner_initial();
    let mut board2 = board1; // Copy

    assert_eq!(board1, board2);

    // Modifying the copy should not affect the original
    board2.set_piece(Square::C3, Some(Piece::new(Color::White, PieceType::Pawn)));
    assert_ne!(board1, board2);
    assert_eq!(board1.piece_at(Square::C3), None);
}

#[test]
fn test_board_display() {
    let board = Board::gardner_initial();
    let display_str = format!("{board}");

    let expected = format!(
        concat!(
            " +---+---+---+---+---+\n",
            " | r | n | b | q | k | 5\n",
            " +---+---+---+---+---+\n",
            " | p | p | p | p | p | 4\n",
            " +---+---+---+---+---+\n",
            " |   |   |   |   |   | 3\n",
            " +---+---+---+---+---+\n",
            " | P | P | P | P | P | 2\n",
            " +---+---+---+---+---+\n",
            " | R | N | B | Q | K | 1\n",
            " +---+---+---+---+---+\n",
            "   a   b   c   d   e\n",
            "\nFen: rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1\n",
            "Key: {:016X}\n",
        ),
        board.key()
    );

    assert_eq!(display_str, expected);
}

#[test]
fn test_make_move_sequence() {
    let mut board = Board::gardner_initial();

    // 1. White plays c2c3
    let m1 = Move::new(Square::C2, Square::C3, None);
    board.make_move(m1);
    assert_eq!(board.piece_at(Square::C2), None);
    assert_eq!(
        board.piece_at(Square::C3),
        Some(Piece::new(Color::White, PieceType::Pawn))
    );
    assert_eq!(board.side_to_move(), Color::Black);

    // 2. Black captures with d4c3
    let m2 = Move::new(Square::D4, Square::C3, None);
    board.make_move(m2);
    assert_eq!(board.piece_at(Square::D4), None);
    assert_eq!(
        board.piece_at(Square::C3),
        Some(Piece::new(Color::Black, PieceType::Pawn))
    );
    assert_eq!(board.side_to_move(), Color::White);
}

#[test]
fn test_make_move_promotion() {
    let mut board = Board::empty();
    // Put a white pawn on E4
    board.set_piece(Square::E4, Some(Piece::new(Color::White, PieceType::Pawn)));

    // Promote E4 -> E5 to Queen
    let m = Move::new(Square::E4, Square::E5, Some(PieceType::Queen));
    board.make_move(m);

    assert_eq!(board.piece_at(Square::E4), None);
    assert_eq!(
        board.piece_at(Square::E5),
        Some(Piece::new(Color::White, PieceType::Queen))
    );
    assert_eq!(board.side_to_move(), Color::Black);
}

#[test]
fn test_make_move_empty_source_is_noop() {
    let mut board = Board::gardner_initial();
    let before = board;

    // C3 is empty, moving from C3 should do nothing
    let m = Move::new(Square::C3, Square::C4, None);
    board.make_move(m);

    assert_eq!(board, before);
}

#[test]
fn test_board_occupied() {
    let empty = Board::empty();
    assert!(empty.occupied().is_empty());
    assert_eq!(empty.occupied().count(), 0);

    let initial = Board::gardner_initial();
    assert_eq!(initial.occupied().count(), 20);

    // Rank 3 (A3..E3) is entirely empty in initial position
    assert!(!initial.occupied().contains(Square::A3));
    assert!(!initial.occupied().contains(Square::B3));
    assert!(!initial.occupied().contains(Square::C3));
    assert!(!initial.occupied().contains(Square::D3));
    assert!(!initial.occupied().contains(Square::E3));

    // Rank 1 and 2 are occupied
    assert!(initial.occupied().contains(Square::A1));
    assert!(initial.occupied().contains(Square::C2));

    // Moving a piece updates occupied bitboard
    let mut board = initial;
    let m = Move::new(Square::C2, Square::C3, None);
    board.make_move(m);

    assert!(!board.occupied().contains(Square::C2));
    assert!(board.occupied().contains(Square::C3));
    assert_eq!(board.occupied().count(), 20);

    // Capture reduces occupied count
    let capture_move = Move::new(Square::D4, Square::C3, None);
    board.make_move(capture_move);
    assert!(!board.occupied().contains(Square::D4));
    assert!(board.occupied().contains(Square::C3));
    assert_eq!(board.occupied().count(), 19);
}

#[test]
fn test_board_color_and_piece_type_bitboards() {
    use zest::board::Bitboard;

    let mut board = Board::gardner_initial();
    assert_eq!(board.color_bb(Color::White).count(), 10);
    assert_eq!(board.color_bb(Color::Black).count(), 10);
    assert_eq!(
        board.color_bb(Color::White) | board.color_bb(Color::Black),
        board.occupied()
    );
    assert_eq!(board.piece_type_bb(PieceType::Pawn).count(), 10);
    assert_eq!(
        board.pieces_bb(Color::White, PieceType::King),
        Bitboard::from_square(Square::E1)
    );
    assert_eq!(
        board.pieces_bb(Color::Black, PieceType::Queen),
        Bitboard::from_square(Square::D5)
    );

    // Promote white pawn at A4 capturing black rook at B5
    board.set_piece(Square::A4, Some(Piece::new(Color::White, PieceType::Pawn)));
    board.make_move(Move::new(Square::A4, Square::B5, Some(PieceType::Queen)));

    assert!(
        board
            .pieces_bb(Color::White, PieceType::Queen)
            .contains(Square::B5)
    );
    assert!(
        !board
            .pieces_bb(Color::White, PieceType::Pawn)
            .contains(Square::A4)
    );
    assert!(!board.color_bb(Color::Black).contains(Square::B5));
    assert_eq!(board.pieces_bb(Color::Black, PieceType::Rook).count(), 1);
    assert_eq!(board.pieces_bb(Color::White, PieceType::Queen).count(), 2);
}
