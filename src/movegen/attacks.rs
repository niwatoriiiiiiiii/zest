use crate::board::{Bitboard, Board};
use crate::core::{Color, PieceType, Square};
use crate::movegen::magic::{bishop_attacks, rook_attacks};

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

const KNIGHT_ATTACKS_TABLE: [Bitboard; 25] = init_step_table(&KNIGHT_MOVES);
const KING_ATTACKS_TABLE: [Bitboard; 25] = init_step_table(&KING_MOVES);
const WHITE_PAWN_ATTACKS_TABLE: [Bitboard; 25] = init_step_table(&WHITE_PAWN_ATTACKS);
const BLACK_PAWN_ATTACKS_TABLE: [Bitboard; 25] = init_step_table(&BLACK_PAWN_ATTACKS);

/// Returns `true` if the given `sq` is attacked by any piece of `by_color`.
#[must_use]
pub fn is_square_attacked(board: &Board, sq: Square, by_color: Color) -> bool {
    is_square_attacked_with_occupied(board, sq, by_color, board.occupied())
}

/// Returns `true` if `sq` is attacked by `by_color` using the provided `occupied` bitboard for sliders.
#[must_use]
pub fn is_square_attacked_with_occupied(
    board: &Board,
    sq: Square,
    by_color: Color,
    occupied: Bitboard,
) -> bool {
    let enemies = board.color_bb(by_color);
    let queens = board.piece_type_bb(PieceType::Queen);

    if pawn_attacks(sq, !by_color).intersects(enemies & board.piece_type_bb(PieceType::Pawn)) {
        return true;
    }
    if knight_attacks(sq).intersects(enemies & board.piece_type_bb(PieceType::Knight)) {
        return true;
    }
    if king_attacks(sq).intersects(enemies & board.piece_type_bb(PieceType::King)) {
        return true;
    }
    if bishop_attacks(sq, occupied)
        .intersects(enemies & (board.piece_type_bb(PieceType::Bishop) | queens))
    {
        return true;
    }
    rook_attacks(sq, occupied).intersects(enemies & (board.piece_type_bb(PieceType::Rook) | queens))
}

/// Returns a bitboard of all squares attacked by a knight on the given `sq`.
#[must_use]
pub const fn knight_attacks(sq: Square) -> Bitboard {
    KNIGHT_ATTACKS_TABLE[sq.index()]
}

/// Returns a bitboard of all squares attacked by a king on the given `sq`.
#[must_use]
pub const fn king_attacks(sq: Square) -> Bitboard {
    KING_ATTACKS_TABLE[sq.index()]
}

/// Returns a bitboard of all squares attacked by a pawn of the given `color` on the given `sq`.
#[must_use]
pub const fn pawn_attacks(sq: Square, color: Color) -> Bitboard {
    match color {
        Color::White => WHITE_PAWN_ATTACKS_TABLE[sq.index()],
        Color::Black => BLACK_PAWN_ATTACKS_TABLE[sq.index()],
    }
}

const fn init_step_table(deltas: &[(i8, i8)]) -> [Bitboard; 25] {
    let mut table = [Bitboard::EMPTY; 25];
    let mut i = 0;
    while i < 25 {
        table[i] = step_attacks(Square::ALL[i], deltas);
        i += 1;
    }
    table
}

const fn step_attacks(sq: Square, deltas: &[(i8, i8)]) -> Bitboard {
    let mut bb = Bitboard::empty();
    let file = sq.file().cast_signed();
    let rank = sq.rank().cast_signed();

    let mut i = 0;
    while i < deltas.len() {
        let (df, dr) = deltas[i];
        let to_f = file + df;
        let to_r = rank + dr;

        if to_f >= 0
            && to_r >= 0
            && let Some(target_sq) = Square::new(to_f.cast_unsigned(), to_r.cast_unsigned())
        {
            bb.set(target_sq);
        }

        i += 1;
    }

    bb
}
