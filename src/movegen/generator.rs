use crate::board::{Bitboard, Board};
use crate::core::{Color, Move, PieceType, Square};
use crate::movegen::{
    CheckInfo, MoveList, between, bishop_attacks, is_square_attacked_with_occupied, king_attacks,
    knight_attacks, pawn_attacks, queen_attacks, rook_attacks,
};

/// Generates all legal moves for the current player (`side_to_move`).
///
/// Uses precomputed check and pin analysis (`CheckInfo`) to directly generate
/// legal moves without requiring full-board copying or post-move filtering for non-king pieces.
///
/// # Panics
///
/// Panics if the current board position does not contain a king for `board.side_to_move()`.
#[must_use]
pub fn generate_legal_moves(board: &Board) -> MoveList {
    let mut moves = MoveList::new();
    let us = board.side_to_move();

    let my_king_sq = board
        .pieces_bb(us, PieceType::King)
        .lsb()
        .expect("King must exist");
    let check_info = CheckInfo::compute(board, us, my_king_sq);
    let num_checkers = check_info.checkers.count();

    let friendly_bb = board.color_bb(us);
    let not_friendly = !friendly_bb;

    // King moves are verified against check
    generate_king_moves(&mut moves, board, my_king_sq, us, not_friendly);

    // If double check, ONLY king moves can escape
    if num_checkers >= 2 {
        return moves;
    }

    // In single check, non-king pieces can only block or capture the checker
    // If not in check, non-king pieces can move to any square
    let check_mask = if num_checkers == 1 {
        let mut checkers_copy = check_info.checkers;
        checkers_copy.pop_lsb().map_or(Bitboard::ALL, |checker_sq| {
            between(my_king_sq, checker_sq) | Bitboard::from_square(checker_sq)
        })
    } else {
        Bitboard::ALL
    };

    // Generate moves for other pieces
    let mut other_pieces = friendly_bb ^ Bitboard::from_square(my_king_sq);
    while let Some(sq) = other_pieces.pop_lsb() {
        let Some(piece) = board.piece_at(sq) else {
            continue;
        };

        let is_pinned = check_info.pinned.contains(sq);
        let allowed_ray = if is_pinned {
            check_info.pin_rays[sq.index()]
        } else {
            Bitboard::ALL
        };

        // A piece pinned along one ray can never resolve a check coming from another ray
        if is_pinned && num_checkers == 1 {
            continue;
        }

        let legal_target_mask = check_mask & allowed_ray & not_friendly;

        match piece.piece_type {
            PieceType::Knight => {
                // Pinned knight can never move along linear pin ray
                if !is_pinned {
                    push_moves(&mut moves, sq, knight_attacks(sq) & legal_target_mask);
                }
            }
            PieceType::Bishop => {
                push_moves(
                    &mut moves,
                    sq,
                    bishop_attacks(sq, board.occupied()) & legal_target_mask,
                );
            }
            PieceType::Rook => {
                push_moves(
                    &mut moves,
                    sq,
                    rook_attacks(sq, board.occupied()) & legal_target_mask,
                );
            }
            PieceType::Queen => {
                push_moves(
                    &mut moves,
                    sq,
                    queen_attacks(sq, board.occupied()) & legal_target_mask,
                );
            }
            PieceType::Pawn => {
                generate_pawn_moves(&mut moves, board, sq, us, legal_target_mask);
            }
            PieceType::King => unreachable!(),
        }
    }

    moves
}

fn push_moves(moves: &mut MoveList, from: Square, mut targets: Bitboard) {
    while let Some(to) = targets.pop_lsb() {
        moves.push(Move::new(from, to, None));
    }
}

fn generate_king_moves(
    moves: &mut MoveList,
    board: &Board,
    king_sq: Square,
    us: Color,
    not_friendly: Bitboard,
) {
    let occ_without_king = board.occupied() & !Bitboard::from_square(king_sq);
    let mut targets = king_attacks(king_sq) & not_friendly;
    while let Some(to) = targets.pop_lsb() {
        if !is_square_attacked_with_occupied(board, to, !us, occ_without_king) {
            moves.push(Move::new(king_sq, to, None));
        }
    }
}

fn generate_pawn_moves(
    moves: &mut MoveList,
    board: &Board,
    sq: Square,
    us: Color,
    legal_target_mask: Bitboard,
) {
    let forward_rank = match us {
        Color::White => sq.rank().cast_signed() + 1,
        Color::Black => sq.rank().cast_signed() - 1,
    };
    if (0..5).contains(&forward_rank)
        && let Some(to) = Square::new(sq.file(), forward_rank.cast_unsigned())
        && !board.occupied().contains(to)
        && legal_target_mask.contains(to)
    {
        push_pawn_move(moves, sq, to, us);
    }

    let mut attacks = pawn_attacks(sq, us) & legal_target_mask & board.occupied();
    while let Some(to) = attacks.pop_lsb() {
        push_pawn_move(moves, sq, to, us);
    }
}

fn push_pawn_move(moves: &mut MoveList, from: Square, to: Square, us: Color) {
    let is_promo = match us {
        Color::White => to.rank() == 4,
        Color::Black => to.rank() == 0,
    };

    if is_promo {
        moves.push(Move::new(from, to, Some(PieceType::Queen)));
        moves.push(Move::new(from, to, Some(PieceType::Rook)));
        moves.push(Move::new(from, to, Some(PieceType::Bishop)));
        moves.push(Move::new(from, to, Some(PieceType::Knight)));
    } else {
        moves.push(Move::new(from, to, None));
    }
}
