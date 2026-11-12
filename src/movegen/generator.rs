use crate::board::{Bitboard, Board};
use crate::core::{Color, Move, Piece, PieceType, Square};
use crate::movegen::{
    CheckInfo, MoveList, between, bishop_attacks, is_square_attacked, king_attacks, knight_attacks,
    pawn_attacks, queen_attacks, rook_attacks,
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

    let my_king_sq = find_king(board, us).expect("King must exist");
    let check_info = CheckInfo::compute(board, us, my_king_sq);
    let num_checkers = check_info.checkers.count();

    // King moves are verified against check
    generate_king_moves(&mut moves, board, my_king_sq, us);

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
    for &sq in &Square::ALL {
        if sq == my_king_sq {
            continue;
        }

        if let Some(piece) = board.piece_at(sq)
            && piece.color == us
        {
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

            let legal_target_mask = check_mask & allowed_ray;

            match piece.piece_type {
                PieceType::Knight => {
                    // Pinned knight can never move along linear pin ray
                    if is_pinned {
                        continue;
                    }
                    let mut targets = knight_attacks(sq) & legal_target_mask;
                    while let Some(to) = targets.pop_lsb() {
                        if let Some(target_piece) = board.piece_at(to)
                            && target_piece.color == us
                        {
                            continue;
                        }
                        moves.push(Move::new(sq, to, None));
                    }
                }
                PieceType::Bishop => {
                    let mut targets = bishop_attacks(sq, board.occupied()) & legal_target_mask;
                    while let Some(to) = targets.pop_lsb() {
                        if let Some(target_piece) = board.piece_at(to)
                            && target_piece.color == us
                        {
                            continue;
                        }
                        moves.push(Move::new(sq, to, None));
                    }
                }
                PieceType::Rook => {
                    let mut targets = rook_attacks(sq, board.occupied()) & legal_target_mask;
                    while let Some(to) = targets.pop_lsb() {
                        if let Some(target_piece) = board.piece_at(to)
                            && target_piece.color == us
                        {
                            continue;
                        }
                        moves.push(Move::new(sq, to, None));
                    }
                }
                PieceType::Queen => {
                    let mut targets = queen_attacks(sq, board.occupied()) & legal_target_mask;
                    while let Some(to) = targets.pop_lsb() {
                        if let Some(target_piece) = board.piece_at(to)
                            && target_piece.color == us
                        {
                            continue;
                        }
                        moves.push(Move::new(sq, to, None));
                    }
                }
                PieceType::Pawn => {
                    generate_pawn_moves(&mut moves, board, sq, us, legal_target_mask);
                }
                PieceType::King => unreachable!(),
            }
        }
    }

    moves
}

fn generate_king_moves(moves: &mut MoveList, board: &Board, king_sq: Square, us: Color) {
    let mut targets = king_attacks(king_sq);
    while let Some(to) = targets.pop_lsb() {
        if let Some(target_piece) = board.piece_at(to)
            && target_piece.color == us
        {
            continue;
        }

        let m = Move::new(king_sq, to, None);
        let mut next_board = *board;
        next_board.make_move(m);
        if !is_square_attacked(&next_board, to, !us) {
            moves.push(m);
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
        && board.piece_at(to).is_none()
        && legal_target_mask.contains(to)
    {
        push_pawn_move(moves, sq, to, us);
    }

    let mut attacks = pawn_attacks(sq, us) & legal_target_mask;
    while let Some(to) = attacks.pop_lsb() {
        if let Some(target) = board.piece_at(to)
            && target.color != us
        {
            push_pawn_move(moves, sq, to, us);
        }
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

fn find_king(board: &Board, color: Color) -> Option<Square> {
    Square::ALL
        .iter()
        .find(|&&sq| board.piece_at(sq) == Some(Piece::new(color, PieceType::King)))
        .copied()
}
