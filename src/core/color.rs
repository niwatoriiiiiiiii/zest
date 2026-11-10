/// Player color / side to move.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    /// White player.
    White,

    /// Black player.
    Black,
}

impl Color {
    /// Returns the opposite color.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

impl std::ops::Not for Color {
    type Output = Self;
    fn not(self) -> Self {
        self.opposite()
    }
}
