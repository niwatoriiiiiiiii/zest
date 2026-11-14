use crate::board::{Bitboard, Board};
use crate::core::{Color, Move, PieceType, Square};
use crate::movegen::{attackers_to_with_occupied, bishop_attacks, rook_attacks};

const SEE_ORDER: [PieceType; 6] = [
    PieceType::Pawn,
    PieceType::Bishop,
    PieceType::Knight,
    PieceType::Rook,
    PieceType::Queen,
    PieceType::King,
];

#[inline]
pub(super) const fn see_piece_value(pt: PieceType) -> i32 {
    match pt {
        PieceType::Pawn => 100,
        PieceType::Bishop => 230,
        PieceType::Knight => 240,
        PieceType::Rook => 315,
        PieceType::Queen => 550,
        PieceType::King => 20_000,
    }
}

#[inline]
fn pop_least_valuable_attacker(
    board: &Board,
    side_attackers: Bitboard,
) -> Option<(PieceType, Square)> {
    for pt in SEE_ORDER {
        let candidates = side_attackers & board.piece_type_bb(pt);
        if let Some(sq) = candidates.lsb() {
            return Some((pt, sq));
        }
    }
    None
}

/// Returns `true` if the Static Exchange Evaluation (SEE) of `mv` is at least `threshold`.
#[must_use]
pub(super) fn see_ge(board: &Board, mv: Move, threshold: i32) -> bool {
    let Some(moving) = board.piece_at(mv.from()) else {
        return false;
    };
    let to = mv.to();
    let victim_value = board
        .piece_at(to)
        .map_or(0, |p| see_piece_value(p.piece_type));
    let promo_bonus = mv.promotion().map_or(0, |promo| {
        see_piece_value(promo) - see_piece_value(PieceType::Pawn)
    });

    let mut swap = victim_value + promo_bonus - threshold;
    if swap < 0 {
        return false;
    }

    let occupant_after_first = mv.promotion().unwrap_or(moving.piece_type);
    swap = see_piece_value(occupant_after_first) - swap;
    if swap <= 0 {
        return true;
    }

    let mut occ = (board.occupied() ^ Bitboard::from_square(mv.from())) | Bitboard::from_square(to);
    let mut attackers = (attackers_to_with_occupied(board, to, Color::White, occ)
        | attackers_to_with_occupied(board, to, Color::Black, occ))
        & occ;

    let diagonal_sliders =
        board.piece_type_bb(PieceType::Bishop) | board.piece_type_bb(PieceType::Queen);
    let orthogonal_sliders =
        board.piece_type_bb(PieceType::Rook) | board.piece_type_bb(PieceType::Queen);

    let mut side = moving.color;
    let mut res = 1u8;

    loop {
        side = !side;
        attackers = attackers & occ;
        let stm_attackers = attackers & board.color_bb(side);
        if stm_attackers.is_empty() {
            break;
        }

        let Some((pt, sq)) = pop_least_valuable_attacker(board, stm_attackers) else {
            break;
        };

        if pt == PieceType::King && !(attackers & board.color_bb(!side)).is_empty() {
            break;
        }

        res ^= 1;
        swap = see_piece_value(pt) - swap;
        if (res == 1 && swap <= 0) || (res == 0 && swap < 0) {
            break;
        }

        occ = occ ^ Bitboard::from_square(sq);
        if matches!(pt, PieceType::Pawn | PieceType::Bishop | PieceType::Queen) {
            attackers = attackers | (bishop_attacks(to, occ) & diagonal_sliders);
        }
        if matches!(pt, PieceType::Rook | PieceType::Queen) {
            attackers = attackers | (rook_attacks(to, occ) & orthogonal_sliders);
        }
    }

    res != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(fen: &str) -> Board {
        Board::from_fen(fen).expect("valid FEN")
    }

    fn uci(s: &str) -> Move {
        Move::from_uci(s).expect("valid move")
    }

    #[test]
    fn see_distinguishes_winning_equal_and_losing_captures() {
        // White Queen on b1 captures Black Pawn on b3 defended by Black Pawn on c4.
        let losing = board("1r1k1/2p2/1p3/5/1Q2K w - - 0 1");
        assert!(!see_ge(&losing, uci("b1b3"), 0));
        assert!(see_ge(&losing, uci("b1b3"), -450));

        // White Rook on c1 captures Black Bishop on c3 (230) defended by Rook on c5.
        let winning = board("2r1k/5/2b2/1Q3/2RK1 w - - 0 1");
        assert!(see_ge(&winning, uci("c1c3"), 0));
        assert!(see_ge(&winning, uci("c1c3"), 200));
        assert!(!see_ge(&winning, uci("c1c3"), 250));
    }

    #[test]
    fn see_reveals_xray_attacker_and_respects_king_capture_legality() {
        // White has Knight on c2 and Rook on c1 behind it, attacking Pawn on c3 defended by Black Pawn on d4.
        let xray = board("4k/3p1/2p2/2N2/2R1K w - - 0 1");
        // N(240)xP(100), P(100)xN -> R(315)xP -> White nets 100 - 240 + 100 = -40.
        assert!(!see_ge(&xray, uci("c2c3"), 0));
        assert!(see_ge(&xray, uci("c2c3"), -40));

        // Black King on d4 cannot recapture on c3 while White Pawn on b2 defends c3.
        let king_blocked = board("5/3k1/2p2/1P3/2R1K w - - 0 1");
        assert!(see_ge(&king_blocked, uci("c1c3"), 100));
        assert!(!see_ge(&king_blocked, uci("c1c3"), 101));

        // Black King on d4 can legally recapture on c3 when no other White piece defends c3.
        let king_legal = board("5/3k1/2n2/1P3/4K w - - 0 1");
        // b2c3: P(100)xN(240), KxP -> -100, net +140.
        assert!(see_ge(&king_legal, uci("b2c3"), 140));
        assert!(!see_ge(&king_legal, uci("b2c3"), 141));
    }
}
