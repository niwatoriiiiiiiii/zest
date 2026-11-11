use crate::core::Color;

/// Types of pieces in Gardner minichess.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PieceType {
    /// Pawn piece.
    Pawn,

    /// Knight piece.
    Knight,

    /// Bishop piece.
    Bishop,

    /// Rook piece.
    Rook,

    /// Queen piece.
    Queen,

    /// King piece.
    King,
}

impl PieceType {
    /// Returns the piece type as a character.
    #[must_use]
    pub const fn to_char(self) -> char {
        match self {
            Self::Pawn => 'p',
            Self::Knight => 'n',
            Self::Bishop => 'b',
            Self::Rook => 'r',
            Self::Queen => 'q',
            Self::King => 'k',
        }
    }
}

/// A piece in Gardner minichess.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Piece {
    /// The color of the piece.
    pub color: Color,

    /// The type of the piece.
    pub piece_type: PieceType,
}

impl Piece {
    /// Creates a piece from a `color` and `piece_type`.
    #[must_use]
    pub const fn new(color: Color, piece_type: PieceType) -> Self {
        Self { color, piece_type }
    }

    /// Returns the piece type and color as a character.
    #[must_use]
    pub const fn to_char(self) -> char {
        match self.color {
            Color::White => self.piece_type.to_char().to_ascii_uppercase(),
            Color::Black => self.piece_type.to_char(),
        }
    }
}
