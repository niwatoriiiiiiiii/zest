use crate::board::Bitboard;
use crate::core::{Color, Move, Piece, PieceType, Square};

/// Represents the state of a 5x5 Gardner minichess board.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board {
    pieces: [Option<Piece>; 25],
    occupied: Bitboard,
    side_to_move: Color,
}

impl Board {
    /// Creates an empty board with white to move.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            pieces: [None; 25],
            occupied: Bitboard::EMPTY,
            side_to_move: Color::White,
        }
    }

    /// Returns the piece at the given `sq`, or `None` if the square is empty.
    #[must_use]
    pub const fn piece_at(self, sq: Square) -> Option<Piece> {
        self.pieces[sq.index()]
    }

    /// Places a `piece` on the given `sq`, or clears the square if `piece` is `None`.
    pub const fn set_piece(&mut self, sq: Square, piece: Option<Piece>) {
        self.pieces[sq.index()] = piece;
        if piece.is_some() {
            self.occupied.set(sq);
        } else {
            self.occupied.clear(sq);
        }
    }

    /// Returns the side to move for the current position.
    #[must_use]
    pub const fn side_to_move(self) -> Color {
        self.side_to_move
    }

    /// Creates a board with the Gardner minichess starting position and white to move.
    #[must_use]
    pub const fn gardner_initial() -> Self {
        let mut board = Self::empty();

        // Rank 1: White pieces (R, N, B, Q, K)
        board.set_piece(Square::A1, Some(Piece::new(Color::White, PieceType::Rook)));
        board.set_piece(
            Square::B1,
            Some(Piece::new(Color::White, PieceType::Knight)),
        );
        board.set_piece(
            Square::C1,
            Some(Piece::new(Color::White, PieceType::Bishop)),
        );
        board.set_piece(Square::D1, Some(Piece::new(Color::White, PieceType::Queen)));
        board.set_piece(Square::E1, Some(Piece::new(Color::White, PieceType::King)));

        // Rank 2: White pawns
        board.set_piece(Square::A2, Some(Piece::new(Color::White, PieceType::Pawn)));
        board.set_piece(Square::B2, Some(Piece::new(Color::White, PieceType::Pawn)));
        board.set_piece(Square::C2, Some(Piece::new(Color::White, PieceType::Pawn)));
        board.set_piece(Square::D2, Some(Piece::new(Color::White, PieceType::Pawn)));
        board.set_piece(Square::E2, Some(Piece::new(Color::White, PieceType::Pawn)));

        // Rank 4: Black pawns
        board.set_piece(Square::A4, Some(Piece::new(Color::Black, PieceType::Pawn)));
        board.set_piece(Square::B4, Some(Piece::new(Color::Black, PieceType::Pawn)));
        board.set_piece(Square::C4, Some(Piece::new(Color::Black, PieceType::Pawn)));
        board.set_piece(Square::D4, Some(Piece::new(Color::Black, PieceType::Pawn)));
        board.set_piece(Square::E4, Some(Piece::new(Color::Black, PieceType::Pawn)));

        // Rank 5: Black pieces (R, N, B, Q, K)
        board.set_piece(Square::A5, Some(Piece::new(Color::Black, PieceType::Rook)));
        board.set_piece(
            Square::B5,
            Some(Piece::new(Color::Black, PieceType::Knight)),
        );
        board.set_piece(
            Square::C5,
            Some(Piece::new(Color::Black, PieceType::Bishop)),
        );
        board.set_piece(Square::D5, Some(Piece::new(Color::Black, PieceType::Queen)));
        board.set_piece(Square::E5, Some(Piece::new(Color::Black, PieceType::King)));

        board
    }

    /// Applies a move to the board, updating piece positions, promotions, and the side to move.
    pub fn make_move(&mut self, m: Move) {
        let Some(mut piece) = self.piece_at(m.from()) else {
            return;
        };
        self.set_piece(m.from(), None);

        if let Some(promo_type) = m.promotion() {
            piece.piece_type = promo_type;
        }
        self.set_piece(m.to(), Some(piece));

        self.side_to_move = !self.side_to_move;
    }

    /// Returns a bitboard of all occupied squares on the board.
    #[must_use]
    pub const fn occupied(self) -> Bitboard {
        self.occupied
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rank in (0..5).rev() {
            writeln!(f, " +---+---+---+---+---+")?;

            write!(f, " |")?;
            for file in 0..5 {
                if let Some(sq) = Square::new(file, rank) {
                    let ch = self.piece_at(sq).map_or(' ', Piece::to_char);
                    write!(f, " {ch} |")?;
                }
            }

            writeln!(f, " {}", rank + 1)?;
        }

        writeln!(f, " +---+---+---+---+---+")?;
        writeln!(f, "   a   b   c   d   e")?;

        // TODO: Add Fen: Key: Checkers:

        Ok(())
    }
}
