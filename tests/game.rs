#![allow(missing_docs)]

use zest::board::{Board, FenError};
use zest::core::{Color, Move, ParseMoveError, Square};
use zest::game::{DrawReason, Game, GameStatus, PlayMoveError};

#[test]
fn test_game_initial_state_and_clocks() {
    let mut game = Game::new();
    assert_eq!(game.status(), GameStatus::Ongoing);
    assert_eq!(game.halfmove_clock(), 0);
    assert_eq!(game.fullmove_number(), 1);
    assert_eq!(game.history(), &[Board::gardner_initial().key()]);
    assert_eq!(game.legal_moves().len(), 7);

    // Non-pawn move increments halfmove clock (knight b1->c3)
    assert_eq!(game.try_uci("b1c3"), Ok(GameStatus::Ongoing));
    assert_eq!(game.halfmove_clock(), 1);
    assert_eq!(game.fullmove_number(), 1);

    // Black knight move increments halfmove clock and fullmove number after Black plays
    assert_eq!(game.try_uci("b5a3"), Ok(GameStatus::Ongoing));
    assert_eq!(game.halfmove_clock(), 2);
    assert_eq!(game.fullmove_number(), 2);

    // Pawn move resets halfmove clock
    assert_eq!(game.try_uci("c2b3"), Err(PlayMoveError::IllegalMove));
    assert_eq!(game.try_uci("d2d3"), Ok(GameStatus::Ongoing));
    assert_eq!(game.halfmove_clock(), 0);
    assert_eq!(game.fullmove_number(), 2);
}

#[test]
fn test_game_checkmate_via_uci() {
    // Black King at e5, pawns at d4/e4; Bishop at b3 covers d5, Rook a2->a5# delivers mate.
    let board = Board::from_fen("4k/3pp/1B3/R4/4K w - - 0 1").expect("valid FEN");
    let mut game = Game::from_board(board);
    assert_eq!(game.status(), GameStatus::Ongoing);

    let status = game.try_uci("a2a5").expect("Ra5# should be legal");
    assert_eq!(status, GameStatus::Checkmate(Color::White));
    assert_eq!(game.status(), GameStatus::Checkmate(Color::White));

    // Subsequent moves are rejected with GameOver
    assert_eq!(
        game.try_uci("d4d3"),
        Err(PlayMoveError::GameOver(GameStatus::Checkmate(Color::White)))
    );
}

#[test]
fn test_game_stalemate() {
    // Black King at e5 has no legal moves and is not in check against Queen at c4 and King at e1.
    let board = Board::from_fen("4k/2Q2/5/5/4K b - - 0 1").expect("valid stalemate FEN");
    let mut game = Game::from_board(board);

    assert_eq!(game.status(), GameStatus::Stalemate);
    assert_eq!(
        game.try_uci("e5d5"),
        Err(PlayMoveError::GameOver(GameStatus::Stalemate))
    );
}

#[test]
fn test_game_insufficient_material() {
    for fen in [
        "4k/5/5/5/4K w - - 0 1",     // K vs K
        "4k/5/2N2/5/4K w - - 0 1",   // K+N vs K
        "4k/5/2b2/5/4K w - - 0 1",   // K vs K+B
        "4k/1b3/5/3B1/4K w - - 0 1", // K+B vs K+B on same square color (b4=(1,3)->0, d2=(3,1)->0)
    ] {
        let board = Board::from_fen(fen).expect("valid FEN");
        let game = Game::from_board(board);
        assert!(game.is_insufficient_material(), "expected draw for {fen}");
        assert_eq!(
            game.status(),
            GameStatus::Draw(DrawReason::InsufficientMaterial)
        );
    }

    // Opposite-colored bishops, K+P vs K, K+R vs K are NOT insufficient material
    for fen in [
        "4k/1b3/5/2B2/4K w - - 0 1", // b4 (color 0) vs c2 (2,1 -> color 1)
        "4k/5/5/4P/4K w - - 0 1",
        "4k/5/5/4R/4K w - - 0 1",
    ] {
        let board = Board::from_fen(fen).expect("valid FEN");
        let game = Game::from_board(board);
        assert!(
            !game.is_insufficient_material(),
            "did not expect insufficient material for {fen}"
        );
        assert_eq!(game.status(), GameStatus::Ongoing);
    }
}

#[test]
fn test_game_threefold_repetition() {
    let mut game = Game::new();

    // Repeat knight shuffles: (Nb1-c3, Nb5-a3, Nc3-b1, Na3-b5) twice to reach initial position 3rd time
    for _ in 0..2 {
        assert_eq!(game.try_uci("b1c3"), Ok(GameStatus::Ongoing));
        assert_eq!(game.try_uci("b5a3"), Ok(GameStatus::Ongoing));
        assert_eq!(game.try_uci("c3b1"), Ok(GameStatus::Ongoing));
        let status = game.try_uci("a3b5").expect("valid knight return");
        if game.is_threefold_repetition() {
            assert_eq!(status, GameStatus::Draw(DrawReason::ThreefoldRepetition));
        } else {
            assert_eq!(status, GameStatus::Ongoing);
        }
    }

    assert!(game.is_threefold_repetition());
    assert_eq!(
        game.status(),
        GameStatus::Draw(DrawReason::ThreefoldRepetition)
    );
}

#[test]
fn test_game_fifty_move_rule() {
    // Blocked pawn columns keep both camps separated; restricting White to ranks 1-2
    // and Black to rank 5 yields hundreds of quiet states without captures or pawn moves.
    let board = Board::from_fen("r3k/p1p1p/P1P1P/5/RN2K w - - 0 1").expect("valid FEN");
    let mut game = Game::from_board(board);

    for ply in 0..100 {
        assert_eq!(
            game.status(),
            GameStatus::Ongoing,
            "ended early at ply {ply}"
        );
        let us = game.board().side_to_move();
        let moves = game.legal_moves();
        let mut quiet_non_mate = moves.iter().copied().filter(|&m| {
            let stays_in_camp = match us {
                Color::White => m.to().rank() <= 1,
                Color::Black => m.to().rank() == 4,
            };
            if !stays_in_camp || game.board().piece_at(m.to()).is_some() {
                return false;
            }
            let mut next = *game.board();
            next.make_move(m);
            !zest::movegen::in_check(&next)
                && !zest::movegen::generate_legal_moves(&next).is_empty()
        });

        let chosen = quiet_non_mate
            .clone()
            .find(|&m| {
                let mut next = *game.board();
                next.make_move(m);
                !game.history().contains(&next.key())
            })
            .or_else(|| {
                quiet_non_mate.find(|&m| {
                    let mut next = *game.board();
                    next.make_move(m);
                    let next_key = next.key();
                    game.history().iter().filter(|&&k| k == next_key).count() < 2
                })
            })
            .expect("should find non-repeating quiet move");

        let _ = game.try_move(chosen).expect("legal move should succeed");
    }

    assert_eq!(game.halfmove_clock(), 100);
    assert!(game.is_fifty_move_rule());
    assert_eq!(game.status(), GameStatus::Draw(DrawReason::FiftyMoveRule));
}

#[test]
fn test_game_irreversible_move_resets_repetition_and_clock() {
    let mut game = Game::new();

    // Repeat initial position once (2nd occurrence)
    for uci in ["b1c3", "b5a3", "c3b1", "a3b5"] {
        assert_eq!(game.try_uci(uci), Ok(GameStatus::Ongoing));
    }
    assert_eq!(game.halfmove_clock(), 4);
    assert!(!game.is_threefold_repetition());

    // Pawn push resets clock and repetition window
    assert_eq!(game.try_uci("a2a3"), Ok(GameStatus::Ongoing));
    assert_eq!(game.halfmove_clock(), 0);

    // Repeat knight shuffles after pawn move: only 2 occurrences of the new post-pawn position
    for uci in ["b5a3", "b1c3", "a3b5", "c3b1"] {
        assert_eq!(game.try_uci(uci), Ok(GameStatus::Ongoing));
    }
    assert!(!game.is_threefold_repetition());
    assert_eq!(game.status(), GameStatus::Ongoing);
}

#[test]
fn test_game_try_uci_parse_error() {
    let mut game = Game::new();
    assert_eq!(
        game.try_uci("bad"),
        Err(PlayMoveError::ParseMove(ParseMoveError::InvalidLength))
    );
    assert_eq!(
        game.try_move(Move::new(Square::A1, Square::A3, None)),
        Err(PlayMoveError::IllegalMove)
    );
}

#[test]
fn test_game_fen_roundtrip_and_clocks() {
    let mut game = Game::new();
    assert_eq!(game.to_fen(), "rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1");

    assert_eq!(game.try_uci("b1c3"), Ok(GameStatus::Ongoing));
    assert_eq!(game.to_fen(), "rnbqk/ppppp/2N2/PPPPP/R1BQK b - - 1 1");

    assert_eq!(game.try_uci("b5a3"), Ok(GameStatus::Ongoing));
    assert_eq!(game.to_fen(), "r1bqk/ppppp/n1N2/PPPPP/R1BQK w - - 2 2");

    let restored = Game::from_fen(&game.to_fen()).expect("valid game FEN");
    assert_eq!(restored.board(), game.board());
    assert_eq!(restored.halfmove_clock(), 2);
    assert_eq!(restored.fullmove_number(), 2);
    assert_eq!(restored.to_fen(), game.to_fen());

    assert_eq!(game.try_uci("d2d3"), Ok(GameStatus::Ongoing));
    assert_eq!(game.to_fen(), "r1bqk/ppppp/n1NP1/PPP1P/R1BQK b - - 0 2");

    assert_eq!(Game::from_fen("bad"), Err(FenError::InvalidFormat));
}

#[test]
fn test_game_from_fen_triggers_fifty_move_rule_at_100() {
    let mut game =
        Game::from_fen("3rk/p1p1p/P1P1P/5/R3K b - - 99 50").expect("valid near-draw FEN");
    assert_eq!(game.status(), GameStatus::Ongoing);

    // One quiet Black king move reaches halfmove_clock == 100 and increments fullmove_number
    assert_eq!(
        game.try_uci("d5c5"),
        Ok(GameStatus::Draw(DrawReason::FiftyMoveRule))
    );
    assert_eq!(game.halfmove_clock(), 100);
    assert_eq!(game.fullmove_number(), 51);
    assert_eq!(game.to_fen(), "2r1k/p1p1p/P1P1P/5/R3K w - - 100 51");
}
