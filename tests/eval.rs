#![allow(missing_docs)]

use zest::board::Board;
use zest::core::{Color, Piece, PieceType, Square};
use zest::eval::{MAX_PHASE, TEMPO_BONUS, evaluate, evaluate_breakdown};

fn mirror_board(board: &Board) -> Board {
    let mut mirrored = Board::empty();
    for sq in Square::ALL {
        if let Some(piece) = board.piece_at(sq) {
            let flipped_sq = Square::new(sq.file(), 4 - sq.rank()).expect("valid mirrored square");
            mirrored.set_piece(flipped_sq, Some(Piece::new(!piece.color, piece.piece_type)));
        }
    }
    mirrored.set_side_to_move(!board.side_to_move());
    mirrored
}

#[test]
fn initial_position_is_symmetric_tempo_only() {
    let board = Board::gardner_initial();
    let breakdown = evaluate_breakdown(&board);

    assert_eq!(breakdown.white, breakdown.black);
    assert_eq!(breakdown.phase, MAX_PHASE);
    assert_eq!(breakdown.tempo, TEMPO_BONUS);
    assert_eq!(breakdown.total, TEMPO_BONUS);
}

#[test]
fn color_mirror_symmetry_holds() {
    let fens = [
        "rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1",
        "r1bqk/pp1pp/2n2/2P2/RNBQK b - - 0 1",
        "4k/1P3/5/3r1/R3K w - - 0 1",
        "2b1k/p2p1/1N3/P1P2/2Q1K b - - 0 1",
        "3qk/5/2P2/1p3/4K w - - 0 1",
        "2k2/1B1B1/5/2n2/2K2 w - - 0 1",
    ];

    for fen in fens {
        let board = Board::from_fen(fen).expect("valid test fen");
        let mirrored = mirror_board(&board);

        assert_eq!(
            evaluate(&board),
            evaluate(&mirrored),
            "mirror symmetry failed for FEN: {fen}"
        );

        let mut flipped_turn = board;
        flipped_turn.set_side_to_move(!board.side_to_move());
        let eval_orig = evaluate(&board) - TEMPO_BONUS;
        let eval_flip = evaluate(&flipped_turn) - TEMPO_BONUS;
        assert_eq!(eval_orig, -eval_flip, "turn flip failed for FEN: {fen}");
    }
}

#[test]
fn material_value_ordering_matches_gardner_hierarchy() {
    let base = Board::from_fen("4k/5/5/5/K4 w - - 0 1").expect("valid kings fen");
    let base_eval = evaluate(&base);

    let add_white = |piece_type: PieceType| -> i32 {
        let mut b = base;
        b.set_piece(Square::C3, Some(Piece::new(Color::White, piece_type)));
        evaluate(&b) - base_eval
    };

    let dp = add_white(PieceType::Pawn);
    let db = add_white(PieceType::Bishop);
    let dn = add_white(PieceType::Knight);
    let dr = add_white(PieceType::Rook);
    let dq = add_white(PieceType::Queen);

    assert!(dp > 0, "pawn should have positive value: {dp}");
    assert!(db > dp, "bishop ({db}) should exceed pawn ({dp})");
    assert!(dn > dp, "knight ({dn}) should exceed pawn ({dp})");
    assert!(
        dr > dn && dr > db,
        "rook ({dr}) should exceed minor pieces ({dn}, {db})"
    );
    assert!(dq > dr, "queen ({dq}) should exceed rook ({dr})");
}

#[test]
fn passed_pawn_beats_blocked_pawn_and_advancement_increases_eval() {
    let blocked = Board::from_fen("4k/2p2/5/2P2/4K w - - 0 1").expect("blocked fen");
    let passed_r2 = Board::from_fen("4k/5/5/2P2/4K w - - 0 1").expect("passed rank 2");
    let passed_r3 = Board::from_fen("4k/5/2P2/5/4K w - - 0 1").expect("passed rank 3");
    let passed_r4 = Board::from_fen("4k/2P2/5/5/4K w - - 0 1").expect("passed rank 4");

    assert!(evaluate(&passed_r2) > evaluate(&blocked));
    assert!(evaluate(&passed_r3) > evaluate(&passed_r2));
    assert!(evaluate(&passed_r4) > evaluate(&passed_r3));
}

#[test]
fn king_prefers_corner_in_middlegame_and_center_in_endgame() {
    // Full-material middlegame with White king on e1 vs c3
    let mg_safe = Board::from_fen("rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1").expect("mg safe");
    let mg_exposed = Board::from_fen("rnbqk/ppppp/2K2/PPPPP/RNBQ1 w - - 0 1").expect("mg exposed");
    assert!(
        evaluate(&mg_safe) > evaluate(&mg_exposed),
        "middlegame king should prefer e1 over exposed c3"
    );

    // Sparse endgame with White king on c3 vs e1
    let eg_edge = Board::from_fen("4k/5/5/2P2/4K w - - 0 1").expect("eg edge");
    let eg_center = Board::from_fen("4k/5/2K2/2P2/5 w - - 0 1").expect("eg center");
    assert!(
        evaluate(&eg_center) > evaluate(&eg_edge),
        "endgame king should prefer central c3 over edge e1"
    );
}

#[test]
fn rook_values_open_above_semi_open_above_closed_file() {
    // Rook on C1 across closed (c2+c4 pawns), semi-open (c4 pawn only), and open (no pawns on C)
    let closed = Board::from_fen("4k/2p2/5/2P2/2R1K w - - 0 1").expect("closed");
    let semi_open = Board::from_fen("4k/2p2/5/1P3/2R1K w - - 0 1").expect("semi-open");
    let open = Board::from_fen("4k/1p3/5/1P3/2R1K w - - 0 1").expect("open");

    assert!(evaluate(&open) > evaluate(&semi_open));
    assert!(evaluate(&semi_open) > evaluate(&closed));
}

#[test]
fn phase_clamps_cleanly_with_extra_promoted_queens() {
    let extra_queens = Board::from_fen("4k/5/QQQ2/QQ3/4K w - - 0 1").expect("extra queens");
    let breakdown = evaluate_breakdown(&extra_queens);

    assert_eq!(breakdown.phase, MAX_PHASE);
    assert!(breakdown.total > 2000);
}

#[test]
fn doubled_and_isolated_pawns_receive_penalties() {
    // Compare connected adjacent pawns vs isolated pawns (same ranks/count/passed status)
    let connected = Board::from_fen("2k2/5/5/PP3/4K w - - 0 1").expect("connected");
    let isolated_edge = Board::from_fen("2k2/5/5/P3P/4K w - - 0 1").expect("isolated edges");
    assert!(evaluate(&connected) > evaluate(&isolated_edge));

    // Compare doubled/tripled pawns on the same pawn count against less-doubled layouts
    let two_normal = Board::from_fen("4k/pp3/5/PP3/4K w - - 0 1").expect("two normal");
    let two_doubled = Board::from_fen("4k/pp3/P4/P4/4K w - - 0 1").expect("two doubled");
    let three_one_doubled =
        Board::from_fen("4k/pp3/P4/PP3/4K w - - 0 1").expect("three with one doubled");
    let three_tripled = Board::from_fen("4k/pp3/P4/P4/P3K w - - 0 1").expect("three tripled");
    assert!(evaluate(&two_normal) > evaluate(&two_doubled));
    assert!(evaluate(&three_one_doubled) > evaluate(&three_tripled));
}
