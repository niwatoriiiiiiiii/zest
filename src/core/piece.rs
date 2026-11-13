use crate::core::Color;

/// Types of pieces in Gardner minichess.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
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

    /// Parses a piece type from a case-insensitive character (`p`, `n`, `b`, `r`, `q`, `k`).
    #[must_use]
    pub const fn from_char(ch: char) -> Option<Self> {
        match ch.to_ascii_lowercase() {
            'p' => Some(Self::Pawn),
            'n' => Some(Self::Knight),
            'b' => Some(Self::Bishop),
            'r' => Some(Self::Rook),
            'q' => Some(Self::Queen),
            'k' => Some(Self::King),
            _ => None,
        }
    }

    /// Returns the array index (`0..=5`) for this piece type.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::Pawn => 0,
            Self::Knight => 1,
            Self::Bishop => 2,
            Self::Rook => 3,
            Self::Queen => 4,
            Self::King => 5,
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

    /// Parses a piece from a FEN character (`'P'`..`'K'` for white, `'p'`..`'k'` for black).
    #[must_use]
    pub const fn from_char(ch: char) -> Option<Self> {
        let color = if ch.is_ascii_uppercase() {
            Color::White
        } else if ch.is_ascii_lowercase() {
            Color::Black
        } else {
            return None;
        };

        match PieceType::from_char(ch) {
            Some(piece_type) => Some(Self::new(color, piece_type)),
            None => None,
        }
    }
}
