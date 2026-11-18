use std::fmt::Write as _;

use zest::core::Color;
use zest::search::{SearchResult, Wdl, is_mate_score, mate_distance_in_moves};

use crate::analysis::{GameAnalysis, SideSummary};

pub fn format_wdl_json(wdl: Wdl, phase: i32) -> String {
    format!(
        "{{\"win\":{},\"draw\":{},\"loss\":{},\"expected\":{:.6},\"phase\":{}}}",
        wdl.win,
        wdl.draw,
        wdl.loss,
        wdl.expected_score(),
        phase
    )
}

pub fn format_search_json(res: &SearchResult) -> String {
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

pub fn parse_moves_arg(input: &str) -> Vec<String> {
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

pub fn format_analysis_json(analysis: &GameAnalysis) -> String {
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
