use crate::board::Bitboard;
use crate::core::{Color, Square};

const KNIGHT_MOVES: [(i8, i8); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];

const KING_MOVES: [(i8, i8); 8] = [
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
];

const WHITE_PAWN_ATTACKS: [(i8, i8); 2] = [(1, 1), (-1, 1)];
const BLACK_PAWN_ATTACKS: [(i8, i8); 2] = [(1, -1), (-1, -1)];

/// Returns a bitboard of all squares attacked by a knight on the given `sq`.
#[must_use]
pub fn knight_attacks(sq: Square) -> Bitboard {
    step_attacks(sq, &KNIGHT_MOVES)
}

/// Returns a bitboard of all squares attacked by a king on the given `sq`.
#[must_use]
pub fn king_attacks(sq: Square) -> Bitboard {
    step_attacks(sq, &KING_MOVES)
}

/// Returns a bitboard of all squares attacked by a pawn of the given `color` on the given `sq`.
#[must_use]
pub fn pawn_attacks(sq: Square, color: Color) -> Bitboard {
    match color {
        Color::White => step_attacks(sq, &WHITE_PAWN_ATTACKS),
        Color::Black => step_attacks(sq, &BLACK_PAWN_ATTACKS),
    }
}

fn step_attacks(sq: Square, deltas: &[(i8, i8)]) -> Bitboard {
    let mut bb = Bitboard::empty();
    let file = sq.file().cast_signed();
    let rank = sq.rank().cast_signed();

    for (df, dr) in deltas {
        let to_f = file + df;
        let to_r = rank + dr;

        if (0..5).contains(&to_f)
            && (0..5).contains(&to_r)
            && let Some(target_sq) = Square::new(to_f.cast_unsigned(), to_r.cast_unsigned())
        {
            bb.set(target_sq);
        }
    }

    bb
}
