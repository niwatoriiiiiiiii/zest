use crate::board::{Bitboard, Board};
use crate::core::{Color, Piece, PieceType, Square};

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

const ROOK_DIRECTIONS: [(i8, i8); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
const BISHOP_DIRECTIONS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

/// Returns `true` if the given `sq` is attacked by any piece of `by_color`.
#[must_use]
pub fn is_square_attacked(board: &Board, sq: Square, by_color: Color) -> bool {
    if has_attacker(
        board,
        pawn_attacks(sq, !by_color),
        by_color,
        PieceType::Pawn,
    ) {
        return true;
    }
    if has_attacker(board, knight_attacks(sq), by_color, PieceType::Knight) {
        return true;
    }
    if has_attacker(board, king_attacks(sq), by_color, PieceType::King) {
        return true;
    }
    if has_slider_attacker(
        board,
        bishop_attacks(sq, board.occupied()),
        by_color,
        PieceType::Bishop,
    ) {
        return true;
    }
    if has_slider_attacker(
        board,
        rook_attacks(sq, board.occupied()),
        by_color,
        PieceType::Rook,
    ) {
        return true;
    }

    false
}

fn has_attacker(board: &Board, mut bb: Bitboard, by_color: Color, piece_type: PieceType) -> bool {
    while let Some(s) = bb.pop_lsb() {
        if board.piece_at(s) == Some(Piece::new(by_color, piece_type)) {
            return true;
        }
    }
    false
}

fn has_slider_attacker(
    board: &Board,
    mut bb: Bitboard,
    by_color: Color,
    piece_type: PieceType,
) -> bool {
    while let Some(s) = bb.pop_lsb() {
        if let Some(piece) = board.piece_at(s)
            && piece.color == by_color
            && (piece.piece_type == piece_type || piece.piece_type == PieceType::Queen)
        {
            return true;
        }
    }
    false
}

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

/// Returns a bitboard of all squares attacked by a rook on the given `sq` with the given `occupied` squares.
#[must_use]
pub fn rook_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    slider_attacks(sq, &ROOK_DIRECTIONS, occupied)
}

/// Returns a bitboard of all squares attacked by a bishop on the given `sq` with the given `occupied` squares.
#[must_use]
pub fn bishop_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    slider_attacks(sq, &BISHOP_DIRECTIONS, occupied)
}

/// Returns a bitboard of all squares attacked by a queen on the given `sq` with the given `occupied` squares.
#[must_use]
pub fn queen_attacks(sq: Square, occupied: Bitboard) -> Bitboard {
    rook_attacks(sq, occupied) | bishop_attacks(sq, occupied)
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

fn slider_attacks(sq: Square, deltas: &[(i8, i8)], occupied: Bitboard) -> Bitboard {
    let mut bb = Bitboard::empty();

    for (df, dr) in deltas {
        let mut cur_f = sq.file().cast_signed();
        let mut cur_r = sq.rank().cast_signed();

        loop {
            cur_f += df;
            cur_r += dr;

            if !(0..5).contains(&cur_f) || !(0..5).contains(&cur_r) {
                break;
            }

            if let Some(target_sq) = Square::new(cur_f.cast_unsigned(), cur_r.cast_unsigned()) {
                bb.set(target_sq);

                if occupied.contains(target_sq) {
                    break;
                }
            }
        }
    }

    bb
}
