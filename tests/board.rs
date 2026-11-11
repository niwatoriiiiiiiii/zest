#![allow(missing_docs)]

use zest::board::Board;
use zest::core::{Color, Piece, PieceType, Square};

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

    let expected = concat!(
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
    );

    assert_eq!(display_str, expected);
}
