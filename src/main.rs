//! Zest UCI CLI entry point.

use std::collections::VecDeque;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use zest::search::{SearchResult, Searcher};
use zest::uci::{
    GoParams, ParseCommandError, UciCommand, UciSession, compute_search_limits,
    compute_time_budget_ms, format_bestmove, format_info, parse_command,
};

enum LoopEvent {
    Line(String),
    Eof,
    SearchFinished {
        searcher: Box<Searcher>,
        result: SearchResult,
        elapsed_ms: u64,
    },
}

struct ActiveSearch {
    stop: Arc<AtomicBool>,
    deadline: Option<Instant>,
    infinite: bool,
    handle: JoinHandle<()>,
}

fn main() {
    run_uci_loop();
}

fn run_uci_loop() {
    let (tx, rx) = mpsc::channel();
    spawn_stdin_reader(tx.clone());

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut session = UciSession::new();
    let mut active: Option<ActiveSearch> = None;
    let mut pending_lines: VecDeque<String> = VecDeque::new();
    let mut quit_requested = false;
    let mut stdin_closed = false;

    loop {
        if active.is_none() {
            if quit_requested {
                break;
            }
            if let Some(line) = pending_lines.pop_front() {
                if handle_idle_line(&line, &mut session, &mut active, &tx, &mut out) {
                    break;
                }
                continue;
            }
            if stdin_closed {
                break;
            }
        }

        let event = match active.as_ref().and_then(|search| search.deadline) {
            Some(deadline) => {
                let wait = deadline.saturating_duration_since(Instant::now());
                match rx.recv_timeout(wait) {
                    Ok(ev) => ev,
                    Err(RecvTimeoutError::Timeout) => {
                        if let Some(search) = active.as_mut() {
                            search.stop.store(true, Ordering::Relaxed);
                            search.deadline = None;
                        }
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            None => match rx.recv() {
                Ok(ev) => ev,
                Err(_) => break,
            },
        };

        match event {
            LoopEvent::Line(line) => {
                if active.is_none() {
                    if handle_idle_line(&line, &mut session, &mut active, &tx, &mut out) {
                        break;
                    }
                } else {
                    handle_busy_line(
                        line,
                        &mut active,
                        &mut pending_lines,
                        &mut quit_requested,
                        &mut out,
                    );
                }
            }
            LoopEvent::Eof => {
                stdin_closed = true;
                if let Some(search) = active.as_ref()
                    && search.infinite
                {
                    search.stop.store(true, Ordering::Relaxed);
                }
            }
            LoopEvent::SearchFinished {
                searcher,
                result,
                elapsed_ms,
            } => {
                if let Some(search) = active.take() {
                    let _ = search.handle.join();
                }
                session.restore_searcher(*searcher);
                let _ = writeln!(out, "{}", format_info(&result, elapsed_ms));
                let _ = writeln!(out, "{}", format_bestmove(&result));
                let _ = out.flush();
            }
        }
    }

    if let Some(search) = active.take() {
        search.stop.store(true, Ordering::Relaxed);
        let _ = search.handle.join();
    }
}

fn spawn_stdin_reader(tx: mpsc::Sender<LoopEvent>) {
    thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if tx.send(LoopEvent::Line(line)).is_err() {
                return;
            }
        }
        let _ = tx.send(LoopEvent::Eof);
    });
}

fn handle_idle_line(
    line: &str,
    session: &mut UciSession,
    active: &mut Option<ActiveSearch>,
    tx: &mpsc::Sender<LoopEvent>,
    out: &mut impl Write,
) -> bool {
    let cmd = match parse_command(line) {
        Ok(cmd) => cmd,
        Err(ParseCommandError::EmptyLine) => return false,
        Err(err) => {
            let _ = writeln!(out, "info string {err}");
            let _ = out.flush();
            return false;
        }
    };

    match cmd {
        UciCommand::Quit => return true,
        UciCommand::Go(params) => {
            *active = Some(start_search(session, params, tx.clone()));
        }
        other => {
            for response in session.handle_command_sync(&other) {
                let _ = writeln!(out, "{response}");
            }
            let _ = out.flush();
        }
    }

    false
}

fn handle_busy_line(
    line: String,
    active: &mut Option<ActiveSearch>,
    pending_lines: &mut VecDeque<String>,
    quit_requested: &mut bool,
    out: &mut impl Write,
) {
    match parse_command(&line) {
        Ok(UciCommand::Stop) => {
            if let Some(search) = active.as_mut() {
                search.stop.store(true, Ordering::Relaxed);
                search.deadline = None;
            }
        }
        Ok(UciCommand::Quit) => {
            *quit_requested = true;
            if let Some(search) = active.as_mut() {
                search.stop.store(true, Ordering::Relaxed);
                search.deadline = None;
            }
        }
        Ok(UciCommand::IsReady) => {
            let _ = writeln!(out, "readyok");
            let _ = out.flush();
        }
        Err(ParseCommandError::EmptyLine) => {}
        _ => pending_lines.push_back(line),
    }
}

fn start_search(
    session: &mut UciSession,
    params: GoParams,
    tx: mpsc::Sender<LoopEvent>,
) -> ActiveSearch {
    let board = *session.game().board();
    let history = session.game().history().to_vec();
    let halfmove_clock = session.game().halfmove_clock();
    let limits = compute_search_limits(&params);
    let budget_ms = compute_time_budget_ms(&params, board.side_to_move());
    let deadline = budget_ms.map(|ms| Instant::now() + Duration::from_millis(ms));
    let mut searcher = Box::new(session.take_searcher());
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);

    let handle = thread::spawn(move || {
        let start = Instant::now();
        let result =
            searcher.search_with_stop(&board, &history, halfmove_clock, limits, &worker_stop);
        let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        let _ = tx.send(LoopEvent::SearchFinished {
            searcher,
            result,
            elapsed_ms,
        });
    });

    ActiveSearch {
        stop,
        deadline,
        infinite: params.infinite,
        handle,
    }
}
