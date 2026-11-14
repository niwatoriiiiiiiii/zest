#![allow(missing_docs)]

use core::sync::atomic::AtomicBool;

use zest::core::Color;
use zest::search::SearchLimits;
use zest::uci::{
    GoParams, ParseCommandError, PositionSpec, UciCommand, UciSession, compute_search_limits,
    compute_time_budget_ms, format_bestmove, format_info, parse_command,
};

#[test]
fn parses_standard_and_debug_commands() {
    assert_eq!(parse_command("  uci "), Ok(UciCommand::Uci));
    assert_eq!(parse_command("isready"), Ok(UciCommand::IsReady));
    assert_eq!(parse_command("ucinewgame"), Ok(UciCommand::UciNewGame));
    assert_eq!(parse_command("eval"), Ok(UciCommand::Eval));
    assert_eq!(parse_command("d"), Ok(UciCommand::Display));
    assert_eq!(parse_command("stop"), Ok(UciCommand::Stop));
    assert_eq!(parse_command("quit"), Ok(UciCommand::Quit));
    assert_eq!(parse_command("go perft 3"), Ok(UciCommand::GoPerft(3)));

    assert_eq!(
        parse_command("setoption name UCI_Variant value gardner"),
        Ok(UciCommand::SetOption {
            name: "UCI_Variant".to_string(),
            value: Some("gardner".to_string()),
        })
    );
    assert_eq!(
        parse_command("setoption name Clear Hash"),
        Ok(UciCommand::SetOption {
            name: "Clear Hash".to_string(),
            value: None,
        })
    );
}

#[test]
fn parses_position_and_go_variants() {
    assert_eq!(
        parse_command("position startpos moves c2c3 d4c3"),
        Ok(UciCommand::Position {
            spec: PositionSpec::Startpos,
            moves: vec!["c2c3".to_string(), "d4c3".to_string()],
        })
    );
    assert_eq!(
        parse_command("position fen 4k/5/5/5/R3K w - - moves a1a2"),
        Ok(UciCommand::Position {
            spec: PositionSpec::Fen("4k/5/5/5/R3K w - -".to_string()),
            moves: vec!["a1a2".to_string()],
        })
    );

    assert_eq!(
        parse_command("go depth 5 nodes 1000 movetime 250 infinite"),
        Ok(UciCommand::Go(GoParams {
            depth: Some(5),
            nodes: Some(1000),
            movetime: Some(250),
            infinite: true,
            ..GoParams::default()
        }))
    );
    assert_eq!(
        parse_command("go wtime 5000 btime 4000 winc 50 binc 100 movestogo 12"),
        Ok(UciCommand::Go(GoParams {
            wtime: Some(5000),
            btime: Some(4000),
            winc: Some(50),
            binc: Some(100),
            movestogo: Some(12),
            ..GoParams::default()
        }))
    );
}

#[test]
fn rejects_malformed_commands() {
    assert_eq!(parse_command("   \t "), Err(ParseCommandError::EmptyLine));
    assert!(matches!(
        parse_command("bogus"),
        Err(ParseCommandError::UnknownCommand(_))
    ));
    assert!(matches!(
        parse_command("setoption value gardner"),
        Err(ParseCommandError::InvalidSyntax(_))
    ));
    assert!(matches!(
        parse_command("position"),
        Err(ParseCommandError::InvalidSyntax(_))
    ));
    assert!(matches!(
        parse_command("position fen"),
        Err(ParseCommandError::InvalidSyntax(_))
    ));
    assert!(matches!(
        parse_command("go depth nope"),
        Err(ParseCommandError::InvalidSyntax(_))
    ));
    assert!(matches!(
        parse_command("go perft 2 extra"),
        Err(ParseCommandError::InvalidSyntax(_))
    ));
}

#[test]
fn applies_position_with_four_field_fen_and_keeps_state_on_error() {
    let mut session = UciSession::new();
    session
        .apply_position(
            &PositionSpec::Fen("4k/5/5/5/R3K w - -".to_string()),
            &["a1a2".to_string()],
        )
        .expect("4-field FEN and legal move should apply");

    let expected_fen = session.game().to_fen();
    assert_eq!(expected_fen, "4k/5/5/R4/4K b - - 1 1");

    let err = session.apply_position(&PositionSpec::Startpos, &["a1e5".to_string()]);
    assert!(err.is_err());
    assert_eq!(session.game().to_fen(), expected_fen);

    session.reset();
    assert_eq!(
        session.game().to_fen(),
        "rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1"
    );
}

#[test]
fn computes_search_limits_and_time_budgets() {
    let bare = GoParams::default();
    assert_eq!(compute_search_limits(&bare), SearchLimits::depth(4));
    assert_eq!(compute_time_budget_ms(&bare, Color::White), None);

    let infinite = GoParams {
        infinite: true,
        movetime: Some(100),
        ..GoParams::default()
    };
    assert_eq!(compute_search_limits(&infinite), SearchLimits::default());
    assert_eq!(compute_time_budget_ms(&infinite, Color::White), None);

    let movetime = GoParams {
        movetime: Some(120),
        ..GoParams::default()
    };
    assert_eq!(compute_search_limits(&movetime), SearchLimits::default());
    assert_eq!(compute_time_budget_ms(&movetime, Color::Black), Some(120));

    let clocks = GoParams {
        wtime: Some(2000),
        btime: Some(4000),
        winc: Some(50),
        binc: Some(100),
        ..GoParams::default()
    };
    assert_eq!(compute_time_budget_ms(&clocks, Color::White), Some(135));
    assert_eq!(compute_time_budget_ms(&clocks, Color::Black), Some(273));
}

#[test]
fn executes_uci_handshake_display_eval_and_perft() {
    let mut session = UciSession::new();

    let uci_lines = session.handle_command_sync(&UciCommand::Uci);
    assert_eq!(uci_lines.first().map(String::as_str), Some("id name Zest"));
    assert_eq!(uci_lines.last().map(String::as_str), Some("uciok"));

    assert_eq!(
        session.handle_command_sync(&UciCommand::IsReady),
        vec!["readyok"]
    );

    let display_lines = session.handle_command_sync(&UciCommand::Display);
    assert!(
        display_lines
            .iter()
            .any(|line| line == "Fen: rnbqk/ppppp/5/PPPPP/RNBQK w - - 0 1")
    );

    let eval_lines = session.handle_command_sync(&UciCommand::Eval);
    assert!(
        eval_lines
            .last()
            .is_some_and(|line| line.starts_with("Final evaluation"))
    );

    let perft_lines = session.handle_command_sync(&UciCommand::GoPerft(2));
    assert_eq!(
        perft_lines.last().map(String::as_str),
        Some("Nodes searched: 53")
    );
}

#[test]
fn formats_search_scores_mate_and_terminal_bestmove() {
    let mut session = UciSession::new();

    session
        .apply_position(
            &PositionSpec::Fen("4k/5/4K/5/2R2 w - - 0 1".to_string()),
            &[],
        )
        .expect("valid mate-in-1 FEN");
    let mate_res = session.search_with_stop(
        &GoParams {
            depth: Some(3),
            ..GoParams::default()
        },
        &AtomicBool::new(false),
    );
    let mate_info = format_info(&mate_res, 5);
    assert!(mate_info.contains("score mate 1"), "info was: {mate_info}");
    assert_eq!(format_bestmove(&mate_res), "bestmove c1c5");

    session
        .apply_position(
            &PositionSpec::Fen("4k/4Q/4K/5/5 b - - 0 1".to_string()),
            &[],
        )
        .expect("valid checkmated FEN");
    let terminal_res = session.search_sync(&GoParams {
        depth: Some(2),
        ..GoParams::default()
    });
    let terminal_info = format_info(&terminal_res, 0);
    assert!(
        terminal_info.contains("score mate 0"),
        "info was: {terminal_info}"
    );
    assert_eq!(format_bestmove(&terminal_res), "bestmove (none)");
}
