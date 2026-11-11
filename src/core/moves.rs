use crate::core::{PieceType, Square};

/// Represents a chess move from one square to another, with optional pawn promotion.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Move {
    from: Square,
    to: Square,
    promotion: Option<PieceType>,
}

impl Move {
    /// Creates a new move with a source `from`, target `to`, and optional `promotion`.
    #[must_use]
    pub const fn new(from: Square, to: Square, promotion: Option<PieceType>) -> Self {
        Self {
            from,
            to,
            promotion,
        }
    }

    /// Returns the source square of the move.
    #[must_use]
    pub const fn from(self) -> Square {
        self.from
    }

    /// Returns the target square of the move.
    #[must_use]
    pub const fn to(self) -> Square {
        self.to
    }

    /// Returns the promotion piece type, or `None` if the move is not a promotion.
    #[must_use]
    pub const fn promotion(self) -> Option<PieceType> {
        self.promotion
    }
}
