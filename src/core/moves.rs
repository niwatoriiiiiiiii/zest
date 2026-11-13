use crate::core::{PieceType, Square};

/// Errors that can occur when parsing a UCI move string.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParseMoveError {
    /// The move string is not 4 or 5 bytes long.
    InvalidLength,
    /// The origin square is invalid.
    InvalidFromSquare,
    /// The destination square is invalid.
    InvalidToSquare,
    /// The origin and destination squares are identical.
    SameSquare,
    /// The promotion piece character is invalid (`n`, `b`, `r`, `q` expected).
    InvalidPromotion,
}

impl std::fmt::Display for ParseMoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLength => write!(f, "UCI move must be 4 or 5 characters"),
            Self::InvalidFromSquare => write!(f, "invalid origin square in UCI move"),
            Self::InvalidToSquare => write!(f, "invalid destination square in UCI move"),
            Self::SameSquare => write!(f, "origin and destination squares cannot be identical"),
            Self::InvalidPromotion => write!(f, "invalid promotion piece in UCI move"),
        }
    }
}

impl std::error::Error for ParseMoveError {}

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

    /// Parses a move in UCI coordinate notation (e.g., `"c2c3"` or `"e4e5q"`).
    ///
    /// # Errors
    ///
    /// Returns a [`ParseMoveError`] if the string length, square coordinates, or promotion
    /// piece are invalid, or if the origin and destination squares are the same.
    pub const fn from_uci(s: &str) -> Result<Self, ParseMoveError> {
        let (from_bytes, to_bytes, promo_byte) = match s.as_bytes() {
            [f1, r1, f2, r2] => ([*f1, *r1], [*f2, *r2], None),
            [f1, r1, f2, r2, p] => ([*f1, *r1], [*f2, *r2], Some(*p)),
            _ => return Err(ParseMoveError::InvalidLength),
        };

        let Some(from) = Square::from_ascii(&from_bytes) else {
            return Err(ParseMoveError::InvalidFromSquare);
        };
        let Some(to) = Square::from_ascii(&to_bytes) else {
            return Err(ParseMoveError::InvalidToSquare);
        };
        if from.as_u8() == to.as_u8() {
            return Err(ParseMoveError::SameSquare);
        }

        let promotion = match promo_byte {
            None => None,
            Some(ch) => match PieceType::from_char(ch as char) {
                Some(
                    pt @ (PieceType::Knight
                    | PieceType::Bishop
                    | PieceType::Rook
                    | PieceType::Queen),
                ) => Some(pt),
                _ => return Err(ParseMoveError::InvalidPromotion),
            },
        };

        Ok(Self::new(from, to, promotion))
    }
}

impl std::str::FromStr for Move {
    type Err = ParseMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_uci(s)
    }
}

impl std::fmt::Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from, self.to)?;
        if let Some(piece_type) = self.promotion {
            write!(f, "{}", piece_type.to_char())?;
        }

        Ok(())
    }
}
