#![allow(missing_docs)]

use zest::board::{Board, FenError};
use zest::core::{Color, Move, Piece, PieceType, Square};

#[test]
fn test_to_fen_initial() {
    let board = Board::gardner_initial();
    assert_eq!(board.to_fen(), "rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1");
}

#[test]
fn test_to_fen_empty() {
    let mut board = Board::empty();
    assert_eq!(board.to_fen(), "5/5/5/5/5 w - - 0 1");

    board.set_side_to_move(Color::Black);
    assert_eq!(board.to_fen(), "5/5/5/5/5 b - - 0 1");
}

#[test]
fn test_from_fen_initial() {
    let fen = "rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1";
    let parsed = Board::from_fen(fen).expect("valid initial FEN");
    assert_eq!(parsed, Board::gardner_initial());
}

#[test]
fn test_from_fen_short_form_and_whitespace() {
    let parsed = Board::from_fen("  rnbqk/ppppp/5/PPPPP/RNBQK   w  ").expect("valid short FEN");
    assert_eq!(parsed, Board::gardner_initial());
}

#[test]
fn test_fen_roundtrip_after_moves() {
    let mut board = Board::gardner_initial();
    board.make_move(Move::new(Square::C2, Square::C3, None));
    board.make_move(Move::new(Square::D4, Square::C3, None));

    let fen = board.to_fen();
    assert_eq!(fen, "rnbqk/ppp1p/2p2/PP1PP/RNBQK w - - 0 1");

    let parsed = Board::from_fen(&fen).expect("roundtrip FEN should parse");
    assert_eq!(parsed, board);
    assert_eq!(parsed.key(), board.key());
}

#[test]
fn test_fen_black_to_move_preserves_zobrist_key() {
    let mut board = Board::gardner_initial();
    board.make_move(Move::new(Square::B1, Square::C3, None));

    let fen = board.to_fen();
    assert_eq!(fen, "rnbqk/ppppp/2N2/PPPPP/R1BQK b - - 0 1");

    let parsed = Board::from_fen(&fen).expect("valid black-to-move FEN");
    assert_eq!(parsed, board);
    assert_eq!(parsed.key(), board.key());
}

#[test]
fn test_fen_mixed_empty_and_pieces() {
    let fen = "2k2/1P2r/3Q1/p3P/K4 b - - 0 1";
    let board = Board::from_fen(fen).expect("valid custom FEN");

    assert_eq!(
        board.piece_at(Square::C5),
        Some(Piece::new(Color::Black, PieceType::King))
    );
    assert_eq!(
        board.piece_at(Square::B4),
        Some(Piece::new(Color::White, PieceType::Pawn))
    );
    assert_eq!(
        board.piece_at(Square::E4),
        Some(Piece::new(Color::Black, PieceType::Rook))
    );
    assert_eq!(
        board.piece_at(Square::D3),
        Some(Piece::new(Color::White, PieceType::Queen))
    );
    assert_eq!(
        board.piece_at(Square::A2),
        Some(Piece::new(Color::Black, PieceType::Pawn))
    );
    assert_eq!(
        board.piece_at(Square::E2),
        Some(Piece::new(Color::White, PieceType::Pawn))
    );
    assert_eq!(
        board.piece_at(Square::A1),
        Some(Piece::new(Color::White, PieceType::King))
    );
    assert_eq!(board.side_to_move(), Color::Black);
    assert_eq!(board.to_fen(), fen);
}

#[test]
fn test_from_fen_errors() {
    assert_eq!(Board::from_fen(""), Err(FenError::InvalidFormat));
    assert_eq!(
        Board::from_fen("rnbqk/ppppp/5/PPPPP/RNBQK"),
        Err(FenError::InvalidFormat)
    );
    assert_eq!(
        Board::from_fen("rnbqk/ppppp/5/PPPPP/RNBQK w -"),
        Err(FenError::InvalidFormat)
    );
    assert_eq!(
        Board::from_fen("rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1 extra"),
        Err(FenError::InvalidFormat)
    );

    assert_eq!(
        Board::from_fen("rnbqk/ppppp/5/PPPPP w - - 0 1"),
        Err(FenError::InvalidRankCount)
    );
    assert_eq!(
        Board::from_fen("rnbqk/ppppp/5/PPPPP/RNBQK/5 w - - 0 1"),
        Err(FenError::InvalidRankCount)
    );

    assert_eq!(
        Board::from_fen("rnbq/ppppp/5/PPPPP/RNBQK w - - 0 1"),
        Err(FenError::InvalidFileCount)
    );
    assert_eq!(
        Board::from_fen("rnbqkP/ppppp/5/PPPPP/RNBQK w - - 0 1"),
        Err(FenError::InvalidFileCount)
    );
    assert_eq!(
        Board::from_fen("rnbqk/ppppp/6/PPPPP/RNBQK w - - 0 1"),
        Err(FenError::InvalidFileCount)
    );
    assert_eq!(
        Board::from_fen("rnbqk/ppppp/05/PPPPP/RNBQK w - - 0 1"),
        Err(FenError::InvalidFileCount)
    );
    assert_eq!(
        Board::from_fen("rnbqk//5/PPPPP/RNBQK w - - 0 1"),
        Err(FenError::InvalidFileCount)
    );

    assert_eq!(
        Board::from_fen("rnbqk/ppxpp/5/PPPPP/RNBQK w - - 0 1"),
        Err(FenError::InvalidPiece('x'))
    );

    assert_eq!(
        Board::from_fen("rnbqk/ppppp/5/PPPPP/RNBQK x - - 0 1"),
        Err(FenError::InvalidSideToMove)
    );
}
