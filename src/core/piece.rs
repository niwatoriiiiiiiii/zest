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
        match ch {
            'P' => Some(Self::new(Color::White, PieceType::Pawn)),
            'N' => Some(Self::new(Color::White, PieceType::Knight)),
            'B' => Some(Self::new(Color::White, PieceType::Bishop)),
            'R' => Some(Self::new(Color::White, PieceType::Rook)),
            'Q' => Some(Self::new(Color::White, PieceType::Queen)),
            'K' => Some(Self::new(Color::White, PieceType::King)),
            'p' => Some(Self::new(Color::Black, PieceType::Pawn)),
            'n' => Some(Self::new(Color::Black, PieceType::Knight)),
            'b' => Some(Self::new(Color::Black, PieceType::Bishop)),
            'r' => Some(Self::new(Color::Black, PieceType::Rook)),
            'q' => Some(Self::new(Color::Black, PieceType::Queen)),
            'k' => Some(Self::new(Color::Black, PieceType::King)),
            _ => None,
        }
    }
}
