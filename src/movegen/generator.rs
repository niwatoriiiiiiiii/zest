use crate::board::Board;
use crate::core::{Color, Move, Piece, PieceType, Square};
use crate::movegen::{
    bishop_attacks, is_square_attacked, king_attacks, knight_attacks, pawn_attacks, queen_attacks,
    rook_attacks,
};

/// Generates all legal moves for the current player (`side_to_move`).
///
/// This first generates all pseudo-legal moves for the player's pieces,
/// and then filters out any move that leaves their own king in check.
#[must_use]
pub fn generate_legal_moves(board: &Board) -> Vec<Move> {
    let mut moves = Vec::new();
    let us = board.side_to_move();

    for &sq in &Square::ALL {
        if let Some(piece) = board.piece_at(sq)
            && piece.color == us
        {
            let targets = match piece.piece_type {
                PieceType::Knight => knight_attacks(sq),
                PieceType::King => king_attacks(sq),
                PieceType::Bishop => bishop_attacks(sq, board.occupied()),
                PieceType::Rook => rook_attacks(sq, board.occupied()),
                PieceType::Queen => queen_attacks(sq, board.occupied()),
                PieceType::Pawn => {
                    generate_pawn_moves(&mut moves, board, sq, us);
                    continue;
                }
            };

            let mut targets = targets;
            while let Some(to) = targets.pop_lsb() {
                if let Some(target_piece) = board.piece_at(to)
                    && target_piece.color == us
                {
                    continue;
                }

                moves.push(Move::new(sq, to, None));
            }
        }
    }

    // NOTE: King must exist on the board
    #[allow(clippy::missing_panics_doc)]
    let my_king_sq = find_king(board, us).expect("King must exist");

    moves.retain(|&m| {
        let mut next_board = *board;
        next_board.make_move(m);
        let king_sq = if m.from() == my_king_sq {
            m.to()
        } else {
            my_king_sq
        };
        !is_square_attacked(&next_board, king_sq, !us)
    });

    moves
}

fn generate_pawn_moves(moves: &mut Vec<Move>, board: &Board, sq: Square, us: Color) {
    let forward_rank = match us {
        Color::White => sq.rank().cast_signed() + 1,
        Color::Black => sq.rank().cast_signed() - 1,
    };
    if (0..5).contains(&forward_rank)
        && let Some(to) = Square::new(sq.file(), forward_rank.cast_unsigned())
        && board.piece_at(to).is_none()
    {
        push_pawn_move(moves, sq, to, us);
    }

    let mut attacks = pawn_attacks(sq, us);
    while let Some(to) = attacks.pop_lsb() {
        if let Some(target) = board.piece_at(to)
            && target.color != us
        {
            push_pawn_move(moves, sq, to, us);
        }
    }
}

fn push_pawn_move(moves: &mut Vec<Move>, from: Square, to: Square, us: Color) {
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
