mod analysis;
mod openings;

use std::fmt::Write as _;

use wasm_bindgen::prelude::*;
use zest::board::Board;
use zest::core::Color;
use zest::eval::evaluate;
use zest::game::Game;
use zest::movegen::generate_legal_moves;
use zest::search::{
    DEFAULT_MULTI_PV, SearchLimits, SearchResult, Wdl, board_phase, is_mate_score,
    mate_distance_in_moves,
};
use zest::uci::{UciCommand, UciSession, parse_command};

use crate::analysis::{DEFAULT_ANALYSIS_DEPTH, GameAnalysis, SideSummary, analyze_moves};

/// Reusable WebAssembly wrapper around Zest's UCI session and search engine.
#[wasm_bindgen]
pub struct WasmEngine {
    session: UciSession,
}

impl Default for WasmEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmEngine {
    /// Creates a new `WasmEngine` with default UCI session and transposition table settings.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: UciSession::new(),
        }
    }

    /// Executes a single UCI command line and returns newline-joined response lines.
    ///
    /// Supports `uci`, `isready`, `setoption` (`Hash`, `Clear Hash`, `MultiPV`, `UCI_ShowWDL`),
    /// `ucinewgame`, `position startpos|fen ... moves ...`, `go depth|nodes`, `eval`, and `d`.
    ///
    /// # Errors
    ///
    /// Returns a JS error string if the UCI command cannot be parsed.
    pub fn command(&mut self, line: &str) -> Result<String, JsValue> {
        self.exec_uci_line(line)
            .map_err(|err| JsValue::from_str(&err))
    }

    /// Resizes the internal transposition table in MiB.
    pub fn set_hash_mb(&mut self, hash_mb: usize) {
        let _ = self.session.handle_command_sync(&UciCommand::SetOption {
            name: "Hash".to_string(),
            value: Some(hash_mb.to_string()),
        });
    }

    /// Clears the transposition table and search ordering heuristics.
    pub fn clear_hash(&mut self) {
        let _ = self.session.handle_command_sync(&UciCommand::SetOption {
            name: "Clear Hash".to_string(),
            value: None,
        });
    }

    /// Searches a 5x5 Gardner FEN position and returns a JSON-formatted result string.
    ///
    /// # Errors
    ///
    /// Returns a JS error string if `fen` cannot be parsed.
    pub fn search_fen(
        &mut self,
        fen: &str,
        max_depth: Option<u8>,
        max_nodes: Option<u64>,
        multi_pv: Option<usize>,
    ) -> Result<String, JsValue> {
        let game = Game::from_fen(fen).map_err(|err| JsValue::from_str(&err.to_string()))?;
        let limits = SearchLimits {
            max_depth,
            max_nodes,
            multi_pv: Some(multi_pv.unwrap_or(DEFAULT_MULTI_PV)),
        };
        let mut searcher = self.session.take_searcher();
        let result = searcher.search(game.board(), game.history(), game.halfmove_clock(), limits);
        self.session.restore_searcher(searcher);
        Ok(format_search_json(&result))
    }

    /// Analyzes a full game move sequence (`moves_input` accepts whitespace-separated UCI moves or a JSON string array)
    /// and returns per-ply move classifications along with Accuracy and ACPL summaries as JSON.
    ///
    /// # Errors
    ///
    /// Returns a JS error string if `start_fen` is invalid or any move is illegal.
    pub fn analyze_game(
        &mut self,
        start_fen: &str,
        moves_input: &str,
        depth: Option<u8>,
    ) -> Result<String, JsValue> {
        let owned = parse_moves_arg(moves_input);
        let move_refs: Vec<&str> = owned.iter().map(String::as_str).collect();
        let search_depth = depth.unwrap_or(DEFAULT_ANALYSIS_DEPTH);
        let mut searcher = self.session.take_searcher();
        let res = analyze_moves(start_fen, &move_refs, search_depth, &mut searcher);
        self.session.restore_searcher(searcher);
        let analysis = res.map_err(|err| JsValue::from_str(&err))?;
        Ok(format_analysis_json(&analysis))
    }
}

/// Returns the number of precomputed balanced openings bundled in WASM.
#[wasm_bindgen]
#[must_use]
pub fn opening_count() -> usize {
    openings::opening_count()
}

/// Returns the whitespace-joined UCI moves for the precomputed opening at `index`, or `None` if out of range.
#[wasm_bindgen]
#[must_use]
pub fn opening_at(index: usize) -> Option<String> {
    openings::opening_at(index).map(str::to_string)
}

impl WasmEngine {
    fn exec_uci_line(&mut self, line: &str) -> Result<String, String> {
        let cmd = parse_command(line).map_err(|err| err.to_string())?;
        Ok(self.session.handle_command_sync(&cmd).join("\n"))
    }
}

/// Returns the static evaluation (in centipawns from side-to-move perspective) for `fen`.
///
/// # Errors
///
/// Returns a JS error string if `fen` cannot be parsed.
#[wasm_bindgen]
pub fn eval_fen(fen: &str) -> Result<i32, JsValue> {
    let board = Board::from_fen(fen).map_err(|err| JsValue::from_str(&err.to_string()))?;
    Ok(evaluate(&board))
}

/// Returns a JSON array string of legal UCI moves for `fen`.
///
/// # Errors
///
/// Returns a JS error string if `fen` cannot be parsed.
#[wasm_bindgen]
pub fn legal_moves_uci(fen: &str) -> Result<String, JsValue> {
    let board = Board::from_fen(fen).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let moves = generate_legal_moves(&board);
    let mut out = String::from("[");
    for (idx, mv) in moves.as_slice().iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let _ = write!(out, "\"{mv}\"");
    }
    out.push(']');
    Ok(out)
}

/// Computes WDL permille probabilities and expected score JSON for `score` and `fen`.
///
/// # Errors
///
/// Returns a JS error string if `fen` cannot be parsed.
#[wasm_bindgen]
pub fn wdl_for_fen(fen: &str, score: i32) -> Result<String, JsValue> {
    let board = Board::from_fen(fen).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let phase = board_phase(&board);
    let wdl = Wdl::from_score(score, phase);
    Ok(format_wdl_json(wdl, phase))
}

fn format_wdl_json(wdl: Wdl, phase: i32) -> String {
    format!(
        "{{\"win\":{},\"draw\":{},\"loss\":{},\"expected\":{:.6},\"phase\":{}}}",
        wdl.win,
        wdl.draw,
        wdl.loss,
        wdl.expected_score(),
        phase
    )
}

fn format_search_json(res: &SearchResult) -> String {
    let wdl = Wdl::from_score(res.score, res.phase);
    let best = res
        .best_move
        .map_or_else(|| "null".to_string(), |m| format!("\"{m}\""));
    let mate =
        mate_distance_in_moves(res.score).map_or_else(|| "null".to_string(), |m| m.to_string());

    let mut out = format!(
        "{{\"bestMove\":{best},\"score\":{},\"isMate\":{},\"mateIn\":{mate},\"depth\":{},\"nodes\":{},\"hashfull\":{},\"phase\":{},\"wdl\":{},\"pvs\":[",
        res.score,
        is_mate_score(res.score),
        res.depth,
        res.nodes,
        res.hashfull,
        res.phase,
        format_wdl_json(wdl, res.phase)
    );

    for (idx, line) in res.pvs.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let line_wdl = Wdl::from_score(line.score, res.phase);
        let line_mate = mate_distance_in_moves(line.score)
            .map_or_else(|| "null".to_string(), |m| m.to_string());
        let _ = write!(
            out,
            "{{\"score\":{},\"isMate\":{},\"mateIn\":{line_mate},\"wdl\":{},\"moves\":[",
            line.score,
            is_mate_score(line.score),
            format_wdl_json(line_wdl, res.phase)
        );
        for (m_idx, mv) in line.moves.iter().enumerate() {
            if m_idx > 0 {
                out.push(',');
            }
            let _ = write!(out, "\"{mv}\"");
        }
        out.push_str("]}");
    }

    out.push_str("]}");
    out
}

fn parse_moves_arg(input: &str) -> Vec<String> {
    let trimmed = input.trim();
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        let inner = &trimmed[1..trimmed.len() - 1];
        inner
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim_matches('\'').trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        trimmed.split_whitespace().map(str::to_string).collect()
    }
}

fn format_side_summary_json(s: &SideSummary) -> String {
    format!(
        "{{\"accuracy\":{:.2},\"acpl\":{:.2},\"rmsDExp\":{:.6},\"countedMoves\":{},\"counts\":{{\"Book\":{},\"Forced\":{},\"Brilliant\":{},\"Critical\":{},\"Best\":{},\"Good\":{},\"Inaccuracy\":{},\"Mistake\":{},\"Blunder\":{}}}}}",
        s.accuracy,
        s.acpl,
        s.rms_d_exp,
        s.counted_moves,
        s.book,
        s.forced,
        s.brilliant,
        s.critical,
        s.best,
        s.good,
        s.inaccuracy,
        s.mistake,
        s.blunder
    )
}

fn format_analysis_json(analysis: &GameAnalysis) -> String {
    let mut out = format!(
        "{{\"white\":{},\"black\":{},\"plies\":[",
        format_side_summary_json(&analysis.white),
        format_side_summary_json(&analysis.black)
    );
    for (idx, p) in analysis.plies.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        let side_str = match p.side {
            Color::White => "white",
            Color::Black => "black",
        };
        let nag_json = p
            .tag
            .nag()
            .map_or_else(|| "null".to_string(), |n| n.to_string());
        let mate_json = p
            .mate_in
            .map_or_else(|| "null".to_string(), |m| m.to_string());
        let _ = write!(
            out,
            "{{\"ply\":{},\"side\":\"{side_str}\",\"uci\":\"{}\",\"score\":{},\"isMate\":{},\"mateIn\":{mate_json},\"expected\":{:.6},\"dExp\":{:.6},\"cpLoss\":{},\"ownSafe\":{},\"ownLegal\":{},\"oppSafe\":{},\"oppLegal\":{},\"tag\":\"{}\",\"symbol\":\"{}\",\"nag\":{nag_json}}}",
            p.ply,
            p.uci,
            p.score,
            p.is_mate,
            p.expected,
            p.d_exp,
            p.cp_loss,
            p.own_safe_count,
            p.own_legal_count,
            p.opp_safe_count,
            p.opp_legal_count,
            p.tag.name(),
            p.tag.symbol()
        );
    }
    out.push_str("]}");
    out
}

#[cfg(test)]
mod tests {
    use zest::search::Searcher;

    use super::*;

    #[test]
    fn uci_command_supports_position_moves_multipv_and_wdl() {
        let mut engine = WasmEngine::new();
        assert!(engine.exec_uci_line("uci").expect("uci").contains("uciok"));
        assert_eq!(engine.exec_uci_line("isready").expect("isready"), "readyok");
        engine
            .exec_uci_line("setoption name MultiPV value 2")
            .expect("set MultiPV");
        engine
            .exec_uci_line("setoption name UCI_ShowWDL value true")
            .expect("set UCI_ShowWDL");
        engine
            .exec_uci_line("position startpos moves a2a3 b4a3")
            .expect("position");
        let out = engine.exec_uci_line("go depth 3").expect("go");
        assert!(out.contains("multipv 1"));
        assert!(out.contains("multipv 2"));
        assert!(out.contains(" wdl "));
        assert!(out.contains("bestmove "));
    }

    #[test]
    fn wasm_helpers_produce_valid_json_shapes() {
        let board = Board::gardner_initial();
        let phase = board_phase(&board);
        let wdl = Wdl::from_score(0, phase);
        let wdl_json = format_wdl_json(wdl, phase);
        assert!(wdl_json.contains("\"win\":"));
        assert!(wdl_json.contains("\"draw\":"));
        assert!(wdl_json.contains("\"loss\":"));
        assert!(wdl_json.contains("\"expected\":0.500000"));

        let mut searcher = Searcher::new();
        let res = searcher.search(&board, &[], 0, SearchLimits::depth(3).with_multi_pv(2));
        let search_json = format_search_json(&res);
        assert!(search_json.contains("\"bestMove\":\""));
        assert!(search_json.contains("\"pvs\":["));
        assert!(search_json.contains("\"wdl\":{"));
    }

    #[test]
    fn wasm_analyze_game_and_openings_work() {
        assert_eq!(opening_count(), 165);
        assert!(opening_at(0).is_some());
        assert_eq!(
            parse_moves_arg("[\"a2a3\", \"b4a3\"]"),
            vec!["a2a3", "b4a3"]
        );

        let mut engine = WasmEngine::new();
        let fen = Board::gardner_initial().to_fen();
        let json = engine
            .analyze_game(&fen, "[\"a2a3\",\"b4a3\",\"b2a3\"]", Some(4))
            .expect("analyze_game");
        assert!(json.contains("\"white\":{"));
        assert!(json.contains("\"black\":{"));
        assert!(json.contains("\"plies\":["));
        assert!(json.contains("\"tag\":\"Book\""));
        assert!(json.contains("\"symbol\":\"📖\""));
    }
}
