//! Head-to-head UCI match runner for 5x5 Gardner minichess.

mod game_runner;
mod stats;
mod uci_player;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

use game_runner::{EngineStats, MatchRules, play_pair};
use stats::print_summary;
use uci_player::{SearchSpec, UciPlayer};

struct Config {
    engine_a: PathBuf,
    engine_b: PathBuf,
    name_a: String,
    name_b: String,
    openings_path: PathBuf,
    pairs: usize,
    workers: usize,
    options_a: Vec<(String, String)>,
    options_b: Vec<(String, String)>,
    rules: MatchRules,
}

fn default_config() -> Config {
    Config {
        engine_a: PathBuf::from("target/release/zest.exe"),
        engine_b: PathBuf::from("target/release/zest.exe"),
        name_a: "Zest-A".to_string(),
        name_b: "Zest-B".to_string(),
        openings_path: PathBuf::from("examples/openings.txt"),
        pairs: 165,
        workers: 4,
        options_a: vec![("Hash".to_string(), "32".to_string())],
        options_b: vec![("Hash".to_string(), "32".to_string())],
        rules: MatchRules {
            spec_a: SearchSpec {
                depth: None,
                nodes: None,
                movetime_ms: Some(100),
            },
            spec_b: SearchSpec {
                depth: None,
                nodes: None,
                movetime_ms: Some(100),
            },
            max_plies: 200,
            resign_cp: 0,
            resign_plies: 4,
        },
    }
}

fn parse_option_kv(raw: &str) -> Result<(String, String), String> {
    let (k, v) = raw
        .split_once('=')
        .ok_or_else(|| format!("expected NAME=VALUE option, got: {raw}"))?;
    Ok((k.trim().to_string(), v.trim().to_string()))
}

fn parse_args() -> Result<Config, String> {
    let mut cfg = default_config();
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0usize;
    while i < args.len() {
        let flag = args[i].as_str();
        if flag == "--help" || flag == "-h" {
            print_usage();
            std::process::exit(0);
        }
        let val = args
            .get(i + 1)
            .ok_or_else(|| format!("missing value for {flag}"))?;
        match flag {
            "--engine-a" => cfg.engine_a = PathBuf::from(val),
            "--engine-b" => cfg.engine_b = PathBuf::from(val),
            "--name-a" => cfg.name_a.clone_from(val),
            "--name-b" => cfg.name_b.clone_from(val),
            "--openings" => cfg.openings_path = PathBuf::from(val),
            "--pairs" => cfg.pairs = val.parse().map_err(|_| format!("invalid --pairs: {val}"))?,
            "--workers" => {
                cfg.workers = val
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --workers: {val}"))?
                    .max(1);
            }
            "--movetime-a" => {
                cfg.rules.spec_a.movetime_ms = Some(val.parse().map_err(|_| "invalid movetime-a")?);
            }
            "--movetime-b" => {
                cfg.rules.spec_b.movetime_ms = Some(val.parse().map_err(|_| "invalid movetime-b")?);
            }
            "--nodes-a" => {
                cfg.rules.spec_a.nodes = Some(val.parse().map_err(|_| "invalid nodes-a")?);
                cfg.rules.spec_a.movetime_ms = None;
            }
            "--nodes-b" => {
                cfg.rules.spec_b.nodes = Some(val.parse().map_err(|_| "invalid nodes-b")?);
                cfg.rules.spec_b.movetime_ms = None;
            }
            "--depth-a" => {
                cfg.rules.spec_a.depth = Some(val.parse().map_err(|_| "invalid depth-a")?);
                cfg.rules.spec_a.movetime_ms = None;
            }
            "--depth-b" => {
                cfg.rules.spec_b.depth = Some(val.parse().map_err(|_| "invalid depth-b")?);
                cfg.rules.spec_b.movetime_ms = None;
            }
            "--option-a" => cfg.options_a.push(parse_option_kv(val)?),
            "--option-b" => cfg.options_b.push(parse_option_kv(val)?),
            "--max-plies" => {
                cfg.rules.max_plies = val.parse().map_err(|_| "invalid --max-plies")?;
            }
            "--resign-cp" => {
                cfg.rules.resign_cp = val.parse().map_err(|_| "invalid --resign-cp")?;
            }
            "--resign-plies" => {
                cfg.rules.resign_plies = val.parse().map_err(|_| "invalid --resign-plies")?;
            }
            _ => return Err(format!("unknown argument: {flag}")),
        }
        i += 2;
    }
    Ok(cfg)
}

fn print_usage() {
    println!(
        "Usage: cargo run --release --example match_uci -- [OPTIONS]\n\
         Options:\n  \
         --engine-a PATH      Engine A binary\n  \
         --engine-b PATH      Engine B binary\n  \
         --name-a NAME        Display name A\n  \
         --name-b NAME        Display name B\n  \
         --movetime-a MS      Movetime ms for A (default 100)\n  \
         --movetime-b MS      Movetime ms for B (default 100)\n  \
         --nodes-a N          Fixed nodes for A\n  \
         --nodes-b N          Fixed nodes for B\n  \
         --depth-a D          Fixed depth for A\n  \
         --depth-b D          Fixed depth for B\n  \
         --option-a NAME=VAL  UCI option for A\n  \
         --option-b NAME=VAL  UCI option for B\n  \
         --openings PATH      Opening lines file (default examples/openings.txt)\n  \
         --pairs N            Opening pairs (default 165)\n  \
         --workers N          Parallel workers (default 4)\n  \
         --max-plies N        Max plies before draw (default 200)\n  \
         --resign-cp CP       Resign threshold cp, 0 disables (default 0)\n  \
         --resign-plies N     Consecutive plies for resign (default 4)"
    );
}

fn load_openings(path: &PathBuf, pairs: usize) -> Result<Vec<Vec<String>>, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let base: Vec<Vec<String>> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split_whitespace().map(str::to_string).collect())
        .collect();
    if base.is_empty() {
        return Err("openings file is empty".to_string());
    }
    Ok((0..pairs).map(|i| base[i % base.len()].clone()).collect())
}

fn main() -> Result<(), String> {
    let cfg = parse_args()?;
    let openings = load_openings(&cfg.openings_path, cfg.pairs)?;
    println!(
        "Match: {} ({}) vs {} ({})",
        cfg.name_a,
        cfg.rules.spec_a.describe(),
        cfg.name_b,
        cfg.rules.spec_b.describe()
    );
    println!(
        "Games: {} ({} pairs, workers={})",
        openings.len() * 2,
        openings.len(),
        cfg.workers
    );

    let start = Instant::now();
    let next_idx = Mutex::new(0usize);
    let results = Mutex::new(Vec::with_capacity(openings.len() * 2));
    let tot_a = Mutex::new(EngineStats::default());
    let tot_b = Mutex::new(EngineStats::default());

    thread::scope(|s| -> Result<(), String> {
        let mut handles = Vec::with_capacity(cfg.workers);
        for _ in 0..cfg.workers {
            handles.push(s.spawn(|| -> Result<(), String> {
                let mut pa = UciPlayer::spawn(&cfg.engine_a, &cfg.options_a)?;
                let mut pb = UciPlayer::spawn(&cfg.engine_b, &cfg.options_b)?;
                let mut sa = EngineStats::default();
                let mut sb = EngineStats::default();
                let mut local = Vec::new();
                loop {
                    let idx = {
                        let mut guard = next_idx.lock().map_err(|_| "lock poisoned")?;
                        if *guard >= openings.len() {
                            break;
                        }
                        let cur = *guard;
                        *guard += 1;
                        cur
                    };
                    let pair = play_pair(
                        &openings[idx],
                        cfg.rules,
                        &mut pa,
                        &mut pb,
                        &mut sa,
                        &mut sb,
                    )?;
                    local.extend(pair);
                }
                results.lock().map_err(|_| "lock poisoned")?.extend(local);
                tot_a.lock().map_err(|_| "lock poisoned")?.merge(&sa);
                tot_b.lock().map_err(|_| "lock poisoned")?.merge(&sb);
                Ok(())
            }));
        }
        for h in handles {
            h.join().map_err(|_| "worker panicked".to_string())??;
        }
        Ok(())
    })?;

    let records = results.into_inner().map_err(|_| "lock poisoned")?;
    let sa = tot_a.into_inner().map_err(|_| "lock poisoned")?;
    let sb = tot_b.into_inner().map_err(|_| "lock poisoned")?;
    print_summary(
        &cfg.name_a,
        &cfg.name_b,
        &records,
        &sa,
        &sb,
        start.elapsed().as_secs_f64(),
    );
    Ok(())
}
