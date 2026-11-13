use std::fmt;

use crate::board::Board;
use crate::core::{Piece, Square};
use crate::movegen::checkers;

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..5).rev() {
            writeln!(f, " +---+---+---+---+---+")?;

            write!(f, " |")?;
            for file in 0..5 {
                if let Some(sq) = Square::new(file, rank) {
                    let ch = self.piece_at(sq).map_or(' ', Piece::to_char);
                    write!(f, " {ch} |")?;
                }
            }

            writeln!(f, " {}", rank + 1)?;
        }

        writeln!(f, " +---+---+---+---+---+")?;
        writeln!(f, "   a   b   c   d   e")?;
        writeln!(f)?;
        writeln!(f, "Fen: {}", self.to_fen())?;
        writeln!(f, "Key: {:016X}", self.key())?;

        write!(f, "Checkers:")?;
        let mut checker_bb = checkers(self);
        if checker_bb.is_empty() {
            write!(f, " ")?;
        } else {
            while let Some(sq) = checker_bb.pop_lsb() {
                write!(f, " {sq}")?;
            }
        }
        writeln!(f)?;

        Ok(())
    }
}
