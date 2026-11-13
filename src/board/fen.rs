use std::fmt;

use crate::board::Board;
use crate::core::{Color, Piece, Square};

/// Maximum length of a Gardner 5x5 FEN string (`29` placement chars + `" b - - 0 1"`).
const MAX_GARDNER_FEN_LEN: usize = 39;

/// Errors that can occur when parsing a Gardner 5x5 FEN string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenError {
    /// The FEN string has missing or extra fields.
    InvalidFormat,

    /// The piece placement field does not contain exactly 5 ranks.
    InvalidRankCount,

    /// A rank in the piece placement field does not span exactly 5 files.
    InvalidFileCount,

    /// An unrecognized character appeared in the piece placement field.
    InvalidPiece(char),

    /// The active color field is neither `'w'` nor `'b'`.
    InvalidSideToMove,
}

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "invalid FEN field structure"),
            Self::InvalidRankCount => write!(f, "FEN placement must have exactly 5 ranks"),
            Self::InvalidFileCount => write!(f, "each FEN rank must span exactly 5 files"),
            Self::InvalidPiece(ch) => write!(f, "invalid piece character in FEN: '{ch}'"),
            Self::InvalidSideToMove => write!(f, "side to move must be 'w' or 'b'"),
        }
    }
}

impl std::error::Error for FenError {}

impl Board {
    /// Serializes the board state into a Gardner 5x5 FEN string.
    ///
    /// Castling rights, en passant square, halfmove clock, and fullmove number
    /// are emitted as the fixed suffix `"- - 0 1"`.
    #[must_use]
    pub fn to_fen(&self) -> String {
        let mut fen_str = String::with_capacity(MAX_GARDNER_FEN_LEN);

        for rank in (0..5_u8).rev() {
            let mut empty_count = 0_u8;

            for file in 0..5_u8 {
                if let Some(sq) = Square::new(file, rank) {
                    if let Some(piece) = self.piece_at(sq) {
                        if empty_count > 0 {
                            fen_str.push(char::from(b'0' + empty_count));
                            empty_count = 0;
                        }
                        fen_str.push(piece.to_char());
                    } else {
                        empty_count += 1;
                    }
                }
            }

            if empty_count > 0 {
                fen_str.push(char::from(b'0' + empty_count));
            }
            if rank > 0 {
                fen_str.push('/');
            }
        }

        match self.side_to_move() {
            Color::White => fen_str.push_str(" w - - 0 1"),
            Color::Black => fen_str.push_str(" b - - 0 1"),
        }

        fen_str
    }

    /// Parses a Gardner 5x5 FEN string into a [`Board`].
    ///
    /// Accepts either the 2-field form (`"<placement> <side>"`) or the standard
    /// 6-field form (`"<placement> <side> <castling> <ep> <halfmove> <fullmove>"`).
    ///
    /// # Errors
    ///
    /// Returns a [`FenError`] if the field count is invalid, the board placement does not
    /// have 5 ranks of 5 files each, an unknown piece character is encountered, or the
    /// side to move is neither `'w'` nor `'b'`.
    pub fn from_fen(fen: &str) -> Result<Self, FenError> {
        let mut fields = fen.split_ascii_whitespace();
        let placement = fields.next().ok_or(FenError::InvalidFormat)?;
        let side_str = fields.next().ok_or(FenError::InvalidFormat)?;

        let remaining_fields = fields.count();
        if remaining_fields != 0 && remaining_fields != 4 {
            return Err(FenError::InvalidFormat);
        }

        let side_to_move = match side_str {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(FenError::InvalidSideToMove),
        };

        let mut board = Self::empty();
        let mut rank_count = 0_u8;

        for rank_str in placement.split('/') {
            if rank_count >= 5 {
                return Err(FenError::InvalidRankCount);
            }
            parse_rank(&mut board, rank_str, 4 - rank_count)?;
            rank_count += 1;
        }

        if rank_count != 5 {
            return Err(FenError::InvalidRankCount);
        }

        board.set_side_to_move(side_to_move);
        Ok(board)
    }
}

/// Parses a single FEN rank string (`a`..`e` files) onto `board` at `rank` (`0..=4`).
fn parse_rank(board: &mut Board, rank_str: &str, rank: u8) -> Result<(), FenError> {
    let mut file = 0_u8;

    for ch in rank_str.chars() {
        match ch {
            '1'..='9' => {
                let empty = (ch as u8) - b'0';
                file += empty;
                if file > 5 {
                    return Err(FenError::InvalidFileCount);
                }
            }
            '0' => return Err(FenError::InvalidFileCount),
            _ => {
                let piece = Piece::from_char(ch).ok_or(FenError::InvalidPiece(ch))?;
                let sq = Square::new(file, rank).ok_or(FenError::InvalidFileCount)?;
                board.set_piece(sq, Some(piece));
                file += 1;
            }
        }
    }

    if file != 5 {
        return Err(FenError::InvalidFileCount);
    }

    Ok(())
}
