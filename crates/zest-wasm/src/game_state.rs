use zest::board::Board;
use zest::core::{Color, Move, PieceType, Square};
use zest::game::{Game, GameStatus};
use zest::movegen::{generate_legal_moves, in_check};

#[derive(Clone, Debug)]
pub struct MoveRecord {
    pub uci: String,
    pub san: String,
}

#[derive(Clone, Debug)]
pub struct GameStateSnapshot {
    pub fen: String,
    pub side_to_move: Color,
    pub in_check: bool,
    pub status: GameStatus,
    pub legal_moves: Vec<Move>,
    pub history: Vec<MoveRecord>,
}

const fn san_piece_char(pt: PieceType) -> Option<char> {
    match pt {
        PieceType::Pawn => None,
        PieceType::Knight => Some('N'),
        PieceType::Bishop => Some('B'),
        PieceType::Rook => Some('R'),
        PieceType::Queen => Some('Q'),
        PieceType::King => Some('K'),
    }
}

const fn file_char(sq: Square) -> char {
    (b'a' + sq.file()) as char
}

const fn rank_char(sq: Square) -> char {
    (b'1' + sq.rank()) as char
}

fn format_san(
    board_before: &Board,
    legal_before: &[Move],
    mv: Move,
    board_after: &Board,
    status_after: GameStatus,
) -> String {
    let mut san = String::new();
    let is_capture = board_before.piece_at(mv.to()).is_some();

    if let Some(piece) = board_before.piece_at(mv.from()) {
        if let Some(pch) = san_piece_char(piece.piece_type) {
            san.push(pch);
            let mut same_file = false;
            let mut same_rank = false;
            let mut ambiguous = false;
            for &other in legal_before {
                if other.from() != mv.from()
                    && other.to() == mv.to()
                    && board_before
                        .piece_at(other.from())
                        .is_some_and(|p| p.piece_type == piece.piece_type)
                {
                    ambiguous = true;
                    if other.from().file() == mv.from().file() {
                        same_file = true;
                    }
                    if other.from().rank() == mv.from().rank() {
                        same_rank = true;
                    }
                }
            }
            if ambiguous {
                if !same_file {
                    san.push(file_char(mv.from()));
                } else if !same_rank {
                    san.push(rank_char(mv.from()));
                } else {
                    san.push(file_char(mv.from()));
                    san.push(rank_char(mv.from()));
                }
            }
        } else if is_capture {
            san.push(file_char(mv.from()));
        }
    }

    if is_capture {
        san.push('x');
    }
    san.push(file_char(mv.to()));
    san.push(rank_char(mv.to()));

    if let Some(promo) = mv.promotion()
        && let Some(pch) = san_piece_char(promo)
    {
        san.push('=');
        san.push(pch);
    }

    if matches!(status_after, GameStatus::Checkmate(_)) {
        san.push('#');
    } else if in_check(board_after) {
        san.push('+');
    }

    san
}

/// Applies `moves_uci` starting from `start_fen` and returns the resulting board, status, legal moves, and SAN history.
///
/// # Errors
///
/// Returns an error string if `start_fen` is invalid or any move in `moves_uci` is illegal.
pub fn evaluate_game_state(
    start_fen: &str,
    moves_uci: &[&str],
) -> Result<GameStateSnapshot, String> {
    let mut game = Game::from_fen(start_fen).map_err(|e| e.to_string())?;
    let mut history = Vec::with_capacity(moves_uci.len());

    for (idx, &uci) in moves_uci.iter().enumerate() {
        let board_before = *game.board();
        let legal_before = generate_legal_moves(&board_before);
        let mv = Move::from_uci(uci).map_err(|e| format!("ply {}: {e}", idx + 1))?;
        let status_after = game
            .try_move(mv)
            .map_err(|e| format!("ply {} ({uci}): {e}", idx + 1))?;
        let san = format_san(
            &board_before,
            legal_before.as_slice(),
            mv,
            game.board(),
            status_after,
        );
        history.push(MoveRecord {
            uci: uci.to_string(),
            san,
        });
    }

    let board = game.board();
    let status = game.status();
    let legal_moves = if matches!(status, GameStatus::Ongoing) {
        generate_legal_moves(board).as_slice().to_vec()
    } else {
        Vec::new()
    };

    Ok(GameStateSnapshot {
        fen: game.to_fen(),
        side_to_move: board.side_to_move(),
        in_check: in_check(board),
        status,
        legal_moves,
        history,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_game_state_produces_san_and_checkmate() {
        let fen = Board::gardner_initial().to_fen();
        let moves = [
            "a2a3", "b4a3", "b2a3", "e4e3", "d2e3", "e5e4", "e3d4", "b5d4", "b1c3", "e4e5", "c3d5",
            "e5d5", "c2c3", "c5a3", "d1d4",
        ];
        let snap = evaluate_game_state(&fen, &moves).expect("state");
        assert_eq!(snap.status, GameStatus::Checkmate(Color::White));
        assert!(snap.legal_moves.is_empty());
        assert_eq!(snap.history[0].san, "a3");
        assert_eq!(snap.history[1].san, "bxa3");
        assert_eq!(snap.history[8].san, "Nc3+");
        assert_eq!(snap.history[14].san, "Qxd4#");
    }
}
