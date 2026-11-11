use crate::core::{Color, Move, Piece, PieceType, Square};

/// Represents the state of a 5x5 Gardner minichess board.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board {
    pieces: [Option<Piece>; 25],
    side_to_move: Color,
}

impl Board {
    /// Creates an empty board with white to move.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            pieces: [None; 25],
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

        // A2 ~ E2, A4 ~ E4
        let mut i = 5;
        while i < 10 {
            board.pieces[i] = Some(Piece::new(Color::White, PieceType::Pawn));
            board.pieces[i + 10] = Some(Piece::new(Color::Black, PieceType::Pawn));
            i += 1;
        }

        // A1 ~ E1
        board.pieces[0] = Some(Piece::new(Color::White, PieceType::Rook));
        board.pieces[1] = Some(Piece::new(Color::White, PieceType::Knight));
        board.pieces[2] = Some(Piece::new(Color::White, PieceType::Bishop));
        board.pieces[3] = Some(Piece::new(Color::White, PieceType::Queen));
        board.pieces[4] = Some(Piece::new(Color::White, PieceType::King));

        // A5 ~ E5
        board.pieces[20] = Some(Piece::new(Color::Black, PieceType::Rook));
        board.pieces[21] = Some(Piece::new(Color::Black, PieceType::Knight));
        board.pieces[22] = Some(Piece::new(Color::Black, PieceType::Bishop));
        board.pieces[23] = Some(Piece::new(Color::Black, PieceType::Queen));
        board.pieces[24] = Some(Piece::new(Color::Black, PieceType::King));

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
