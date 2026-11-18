mod analysis;
mod json;
mod openings;

use std::fmt::Write as _;

use wasm_bindgen::prelude::*;
use zest::board::Board;
use zest::eval::evaluate;
use zest::game::Game;
use zest::movegen::generate_legal_moves;
use zest::search::{DEFAULT_MULTI_PV, SearchLimits, Wdl, board_phase};
use zest::uci::{UciCommand, UciSession, parse_command};

use crate::analysis::{DEFAULT_ANALYSIS_DEPTH, analyze_moves};
use crate::json::{format_analysis_json, format_search_json, format_wdl_json, parse_moves_arg};

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
