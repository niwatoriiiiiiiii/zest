use core::ops::{BitAnd, BitOr, BitXor, Not};

use crate::core::Square;

/// Represents a 5x5 board state as a 25-bit mask in a `u32`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct Bitboard(u32);

impl Bitboard {
    /// An empty bitboard with no squares set.
    pub const EMPTY: Self = Self(0);

    /// A bitboard with all 25 squares set.
    pub const ALL: Self = Self((1 << 25) - 1);

    /// Creates an empty bitboard.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Creates a bitboard with only the given `sq` set.
    #[must_use]
    pub const fn from_square(sq: Square) -> Self {
        Self(1u32 << sq.index())
    }

    /// Returns `true` if the given `sq` is set in the bitboard.
    #[must_use]
    pub const fn contains(self, sq: Square) -> bool {
        // AND(self.0, mask) != 0
        (self.0 & (1u32 << sq.index())) != 0
    }

    /// Returns `true` if no squares are set in the bitboard.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns the number of set squares in the bitboard.
    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Sets the bit corresponding to the given `sq`.
    pub const fn set(&mut self, sq: Square) {
        // self.0 = OR(self.0, mask)
        self.0 |= 1u32 << sq.index();
    }

    /// Clears the bit corresponding to the given `sq`.
    pub const fn clear(&mut self, sq: Square) {
        // self.0 = AND(self.0, NOT(mask))
        self.0 &= !(1u32 << sq.index());
    }

    /// Toggles the bit corresponding to the given `sq`.
    pub const fn toggle(&mut self, sq: Square) {
        // self.0 = XOR(self.0, mask)
        self.0 ^= 1u32 << sq.index();
    }

    /// Removes and returns the square of the least significant bit, or `None` if empty.
    pub const fn pop_lsb(&mut self) -> Option<Square> {
        if self.is_empty() {
            return None;
        }

        // NOTE: `trailing_zeros()` on `u32` returns at most 32, which always fits in `u8`.
        #[allow(clippy::cast_possible_truncation)]
        let index = self.0.trailing_zeros() as u8;
        let sq = Square::from_index(index);

        // self.0 = AND(self.0, self.0 - 1)
        self.0 &= self.0 - 1;

        sq
    }

    /// Creates a bitboard from a raw `u32` value (masked to 25 bits).
    #[must_use]
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw & ((1 << 25) - 1))
    }
    /// Returns the raw `u32` bitmask of the bitboard.
    #[must_use]
    pub const fn as_raw(self) -> u32 {
        self.0
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0 & ((1 << 25) - 1))
    }
}
