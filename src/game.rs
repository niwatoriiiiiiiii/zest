use std::fmt;

use crate::board::Board;
use crate::core::{Color, Move, ParseMoveError, PieceType, Square};
use crate::movegen::{MoveList, generate_legal_moves, in_check};

/// Reasons a Gardner minichess game can end in a draw.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DrawReason {
    /// Neither side has enough material to force checkmate.
    InsufficientMaterial,

    /// The same position has occurred three times.
    ThreefoldRepetition,

    /// Fifty consecutive moves by both sides without a pawn move or capture.
    FiftyMoveRule,
}

/// Current state of a Gardner minichess game.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameStatus {
    /// The game is still in progress.
    Ongoing,

    /// The specified color delivered checkmate and won.
    Checkmate(Color),

    /// The side to move has no legal moves and is not in check.
    Stalemate,

    /// The game ended in a draw for the given reason.
    Draw(DrawReason),
}

/// Errors returned when attempting to play a move on a [`Game`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PlayMoveError {
    /// The game has already ended and cannot accept further moves.
    GameOver(GameStatus),

    /// The UCI move string could not be parsed.
    ParseMove(ParseMoveError),

    /// The move is not legal in the current position.
    IllegalMove,
}

impl fmt::Display for PlayMoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GameOver(status) => write!(f, "game is already over: {status:?}"),
            Self::ParseMove(err) => write!(f, "invalid move syntax: {err}"),
            Self::IllegalMove => write!(f, "illegal move for the current position"),
        }
    }
}

impl std::error::Error for PlayMoveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseMove(err) => Some(err),
            Self::GameOver(_) | Self::IllegalMove => None,
        }
    }
}

impl From<ParseMoveError> for PlayMoveError {
    fn from(err: ParseMoveError) -> Self {
        Self::ParseMove(err)
    }
}

/// Manages a Gardner minichess game with move history, clocks, and termination rules.
///
/// Kept separate from [`Board`] so `perft` and search hot paths retain a compact,
/// `Copy` board representation without heap allocations.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Game {
    board: Board,
    halfmove_clock: u16,
    fullmove_number: u16,
    history: Vec<u64>,
    reversible_start: usize,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    /// Creates a new game from the Gardner minichess starting position.
    #[must_use]
    pub fn new() -> Self {
        Self::from_board(Board::gardner_initial())
    }

    /// Creates a game starting from the given `board` with `halfmove_clock = 0` and `fullmove_number = 1`.
    #[must_use]
    pub fn from_board(board: Board) -> Self {
        Self {
            board,
            halfmove_clock: 0,
            fullmove_number: 1,
            history: vec![board.key()],
            reversible_start: 0,
        }
    }

    /// Returns a reference to the current board.
    #[must_use]
    pub const fn board(&self) -> &Board {
        &self.board
    }

    /// Returns the halfmove clock since the last pawn move or capture.
    #[must_use]
    pub const fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }

    /// Returns the 1-based fullmove counter.
    #[must_use]
    pub const fn fullmove_number(&self) -> u16 {
        self.fullmove_number
    }

    /// Returns the sequence of Zobrist position keys encountered from the start of the game.
    #[must_use]
    pub fn history(&self) -> &[u64] {
        &self.history
    }

    /// Generates all legal moves for the current board position.
    #[must_use]
    pub fn legal_moves(&self) -> MoveList {
        generate_legal_moves(&self.board)
    }

    /// Returns `true` if neither side has enough material to deliver checkmate.
    #[must_use]
    pub fn is_insufficient_material(&self) -> bool {
        let heavy_or_pawns = self.board.piece_type_bb(PieceType::Pawn)
            | self.board.piece_type_bb(PieceType::Rook)
            | self.board.piece_type_bb(PieceType::Queen);
        if !heavy_or_pawns.is_empty() {
            return false;
        }

        let total_pieces = self.board.occupied().count();
        if total_pieces <= 3 {
            // K vs K, K+N vs K, K+B vs K
            return true;
        }

        if total_pieces == 4 {
            let white_bishops = self.board.pieces_bb(Color::White, PieceType::Bishop);
            let black_bishops = self.board.pieces_bb(Color::Black, PieceType::Bishop);
            if white_bishops.count() == 1
                && black_bishops.count() == 1
                && let (Some(wb), Some(bb)) = (white_bishops.lsb(), black_bishops.lsb())
            {
                return square_color(wb) == square_color(bb);
            }
        }

        false
    }

    /// Returns `true` if the current position has occurred at least three times since the last irreversible move.
    #[must_use]
    pub fn is_threefold_repetition(&self) -> bool {
        let current = self.board.key();
        self.history[self.reversible_start..]
            .iter()
            .rev()
            .step_by(2)
            .filter(|&&key| key == current)
            .take(3)
            .count()
            >= 3
    }

    /// Returns `true` if the 50-move rule applies (`halfmove_clock >= 100`).
    #[must_use]
    pub const fn is_fifty_move_rule(&self) -> bool {
        self.halfmove_clock >= 100
    }

    /// Evaluates the current status of the game.
    ///
    /// Checkmate and stalemate take precedence over draw conditions if there are no legal moves.
    #[must_use]
    pub fn status(&self) -> GameStatus {
        let moves = generate_legal_moves(&self.board);
        self.status_with_moves(&moves)
    }

    /// Validates and plays `m` if the game is ongoing, returning the updated [`GameStatus`].
    ///
    /// # Errors
    ///
    /// Returns [`PlayMoveError::GameOver`] if the game has already ended, or
    /// [`PlayMoveError::IllegalMove`] if `m` is not legal in the current position.
    pub fn try_move(&mut self, m: Move) -> Result<GameStatus, PlayMoveError> {
        let legal = generate_legal_moves(&self.board);
        let pre_status = self.status_with_moves(&legal);
        if pre_status != GameStatus::Ongoing {
            return Err(PlayMoveError::GameOver(pre_status));
        }

        if !legal.contains(&m) {
            return Err(PlayMoveError::IllegalMove);
        }

        let side_before = self.board.side_to_move();
        let is_pawn_move = self
            .board
            .piece_at(m.from())
            .is_some_and(|piece| piece.piece_type == PieceType::Pawn);
        let is_capture = self.board.piece_at(m.to()).is_some();

        self.board.make_move(m);
        self.history.push(self.board.key());

        if is_pawn_move || is_capture {
            self.halfmove_clock = 0;
            self.reversible_start = self.history.len() - 1;
        } else {
            self.halfmove_clock = self.halfmove_clock.saturating_add(1);
        }

        if side_before == Color::Black {
            self.fullmove_number = self.fullmove_number.saturating_add(1);
        }

        Ok(self.status())
    }

    /// Parses a UCI move string and plays it if legal, returning the updated [`GameStatus`].
    ///
    /// # Errors
    ///
    /// Returns [`PlayMoveError::ParseMove`] if the string is malformed, [`PlayMoveError::GameOver`]
    /// if the game has already ended, or [`PlayMoveError::IllegalMove`] if the move is illegal.
    pub fn try_uci(&mut self, uci: &str) -> Result<GameStatus, PlayMoveError> {
        let m = Move::from_uci(uci)?;
        self.try_move(m)
    }

    fn status_with_moves(&self, legal_moves: &MoveList) -> GameStatus {
        if legal_moves.is_empty() {
            return if in_check(&self.board) {
                GameStatus::Checkmate(!self.board.side_to_move())
            } else {
                GameStatus::Stalemate
            };
        }

        if self.is_insufficient_material() {
            return GameStatus::Draw(DrawReason::InsufficientMaterial);
        }
        if self.is_threefold_repetition() {
            return GameStatus::Draw(DrawReason::ThreefoldRepetition);
        }
        if self.is_fifty_move_rule() {
            return GameStatus::Draw(DrawReason::FiftyMoveRule);
        }

        GameStatus::Ongoing
    }
}

const fn square_color(sq: Square) -> u8 {
    (sq.file() + sq.rank()) % 2
}
