/// Square positions on a chess board (0..=24).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Square(u8);

#[allow(missing_docs)]
impl Square {
    pub const A1: Self = Self(0);
    pub const B1: Self = Self(1);
    pub const C1: Self = Self(2);
    pub const D1: Self = Self(3);
    pub const E1: Self = Self(4);

    pub const A2: Self = Self(5);
    pub const B2: Self = Self(6);
    pub const C2: Self = Self(7);
    pub const D2: Self = Self(8);
    pub const E2: Self = Self(9);

    pub const A3: Self = Self(10);
    pub const B3: Self = Self(11);
    pub const C3: Self = Self(12);
    pub const D3: Self = Self(13);
    pub const E3: Self = Self(14);

    pub const A4: Self = Self(15);
    pub const B4: Self = Self(16);
    pub const C4: Self = Self(17);
    pub const D4: Self = Self(18);
    pub const E4: Self = Self(19);

    pub const A5: Self = Self(20);
    pub const B5: Self = Self(21);
    pub const C5: Self = Self(22);
    pub const D5: Self = Self(23);
    pub const E5: Self = Self(24);

    pub const ALL: [Self; 25] = {
        let mut arr = [Self(0); 25];
        let mut i: u8 = 0;
        while i < 25 {
            arr[i as usize] = Self(i);
            i += 1;
        }
        arr
    };
}

impl Square {
    /// Creates a square from a `file` and `rank`.
    #[must_use]
    pub const fn new(file: u8, rank: u8) -> Option<Self> {
        if file < 5 && rank < 5 {
            Some(Self(rank * 5 + file))
        } else {
            None
        }
    }

    /// Returns the file of the square.
    #[must_use]
    pub const fn file(self) -> u8 {
        self.0 % 5
    }

    /// Returns the rank of the square.
    #[must_use]
    pub const fn rank(self) -> u8 {
        self.0 / 5
    }

    /// Returns the square index as a `usize` (0..=24).
    #[must_use]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Returns the raw square index as a `u8` (0..=24).
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self.0
    }
}
