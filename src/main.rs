//! Zest CLI entry point.

use zest::board::Board;
use zest::core::{Color, PieceType, Square};

fn main() {
    let c = Color::White;
    let sq = Square::new(0, 0).unwrap(); // a1
    println!("Color: {:?}, Opposite: {:?}", c, c.opposite());
    println!("Square: {:?} (file={}, rank={})", sq, sq.file(), sq.rank());
    println!("Piece: {}", PieceType::Pawn.to_char());

    let board = Board::gardner_initial();
    println!("{board}");
}
