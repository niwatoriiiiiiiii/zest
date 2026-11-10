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
