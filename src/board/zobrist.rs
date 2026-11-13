use crate::core::{Piece, Square};

/// Precomputed 64-bit random keys for Zobrist hashing.
pub struct ZobristKeys {
    pieces: [[[u64; 25]; 6]; 2],
    side_to_move: u64,
}

impl ZobristKeys {
    const KEYS: Self = Self::init_keys();

    const fn init_keys() -> Self {
        let mut state = 0x1234_5678_9ABC_DEF0_u64;
        let mut pieces = [[[0u64; 25]; 6]; 2];

        let mut c = 0;
        while c < 2 {
            let mut pt = 0;
            while pt < 6 {
                let mut sq = 0;
                while sq < 25 {
                    pieces[c][pt][sq] = Self::split_mix_64(&mut state);
                    sq += 1;
                }
                pt += 1;
            }
            c += 1;
        }

        let side_to_move = Self::split_mix_64(&mut state);
        Self {
            pieces,
            side_to_move,
        }
    }

    const fn split_mix_64(state: &mut u64) -> u64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Returns the Zobrist key for the given `piece` on `sq`.
    #[must_use]
    pub const fn piece_key(piece: Piece, sq: Square) -> u64 {
        Self::KEYS.pieces[piece.color.index()][piece.piece_type.index()][sq.index()]
    }

    /// Returns the Zobrist key used to toggle the side to move.
    #[must_use]
    pub const fn side_to_move_key() -> u64 {
        Self::KEYS.side_to_move
    }
}
