use crate::core::Square;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub struct Bitboard(u32);

impl Bitboard {
    pub const EMPTY: Self = Self(0);
    pub const ALL: Self = Self((1 << 25) - 1);

    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn from_square(sq: Square) -> Self {
        Self(1u32 << sq.index())
    }

    #[must_use]
    pub const fn contains(self, sq: Square) -> bool {
        // AND(self.0, mask) != 0
        (self.0 & (1u32 << sq.index())) != 0
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    pub const fn set(&mut self, sq: Square) {
        // self.0 = OR(self.0, mask)
        self.0 |= 1u32 << sq.index();
    }

    pub const fn clear(&mut self, sq: Square) {
        // self.0 = AND(self.0, NOT(mask))
        self.0 &= !(1u32 << sq.index());
    }

    pub const fn toggle(&mut self, sq: Square) {
        // self.0 = XOR(self.0, mask)
        self.0 ^= 1u32 << sq.index();
    }

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
}
