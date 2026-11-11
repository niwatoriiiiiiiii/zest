//! Zest CLI entry point.

use zest::board::Board;
use zest::core::{Color, Move, PieceType, Square};

fn main() {
    let c = Color::White;
    let sq = Square::new(0, 0).unwrap(); // a1
    println!("Color: {:?}, Opposite: {:?}", c, c.opposite());
    println!("Square: {:?} (file={}, rank={})", sq, sq.file(), sq.rank());
    println!("Piece: {}", PieceType::Pawn.to_char());

    let mut board = Board::gardner_initial();
    println!("----- Initial position -----");
    println!("{board}");

    // 1. c2c3
    let m1 = Move::new(Square::C2, Square::C3, None);
    println!("White plays: {m1}");
    board.make_move(m1);
    println!("{board}");
    println!("Side to move: {:?}", board.side_to_move());

    println!();
    println!("----------------------------");
    println!();

    // 2. d4c3
    let m2 = Move::new(Square::D4, Square::C3, None);
    println!("Black plays: {m2}");
    board.make_move(m2);
    println!("{board}");
    println!("Side to move: {:?}", board.side_to_move());
}
