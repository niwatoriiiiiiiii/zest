use crate::core::{Move, Square};

/// Maximum number of pseudo-legal moves in a 5x5 Gardner minichess position.
pub const MAX_MOVES: usize = 128;

const DUMMY_MOVE: Move = Move::new(Square::A1, Square::A1, None);

/// Fixed-capacity stack-allocated move buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveList {
    moves: [Move; MAX_MOVES],
    len: usize,
}

impl MoveList {
    /// Creates an empty `MoveList`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            moves: [DUMMY_MOVE; MAX_MOVES],
            len: 0,
        }
    }

    /// Clears the list, resetting its length to 0.
    pub const fn clear(&mut self) {
        self.len = 0;
    }

    /// Returns the number of moves in the list.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the list contains no moves.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Appends a move to the list.
    ///
    /// # Panics
    /// Panics in debug mode if the list exceeds `MAX_MOVES`.
    pub fn push(&mut self, m: Move) {
        debug_assert!(self.len < MAX_MOVES, "MoveList overflow");
        self.moves[self.len] = m;
        self.len += 1;
    }

    /// Returns a slice containing all moves in the list.
    #[must_use]
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }

    /// Returns a mutable slice containing all moves in the list.
    #[must_use]
    pub fn as_mut_slice(&mut self) -> &mut [Move] {
        &mut self.moves[..self.len]
    }

    /// Retains only the elements specified by the predicate.
    pub fn retain<F>(&mut self, mut predicate: F)
    where
        F: FnMut(&Move) -> bool,
    {
        let mut write_idx = 0;
        for read_idx in 0..self.len {
            let m = self.moves[read_idx];
            if predicate(&m) {
                self.moves[write_idx] = m;
                write_idx += 1;
            }
        }
        self.len = write_idx;
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for MoveList {
    type Target = [Move];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl std::ops::DerefMut for MoveList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}
