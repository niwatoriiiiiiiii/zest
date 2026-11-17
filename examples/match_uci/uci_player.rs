use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

#[derive(Clone, Copy, Debug)]
pub struct SearchSpec {
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    pub movetime_ms: Option<u64>,
}

impl SearchSpec {
    pub fn to_go_command(self) -> String {
        let mut parts = vec!["go".to_string()];
        if let Some(d) = self.depth {
            parts.push("depth".to_string());
            parts.push(d.to_string());
        }
        if let Some(n) = self.nodes {
            parts.push("nodes".to_string());
            parts.push(n.to_string());
        }
        if let Some(ms) = self.movetime_ms {
            parts.push("movetime".to_string());
            parts.push(ms.to_string());
        }
        if parts.len() == 1 {
            parts.push("movetime".to_string());
            parts.push("100".to_string());
        }
        parts.join(" ")
    }

    pub fn describe(self) -> String {
        let mut items = Vec::new();
        if let Some(d) = self.depth {
            items.push(format!("depth={d}"));
        }
        if let Some(n) = self.nodes {
            items.push(format!("nodes={n}"));
        }
        if let Some(ms) = self.movetime_ms {
            items.push(format!("movetime={ms}ms"));
        }
        if items.is_empty() {
            "movetime=100ms".to_string()
        } else {
            items.join(", ")
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MoveSearchResult {
    pub bestmove: Option<String>,
    pub score_cp: Option<i32>,
    pub mate_in: Option<i32>,
    pub depth: Option<u64>,
    pub nodes: Option<u64>,
    pub nps: Option<u64>,
}

pub struct UciPlayer {
    child: Child,
    stdin: BufWriter<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}

impl UciPlayer {
    pub fn spawn(exe: &Path, options: &[(String, String)]) -> Result<Self, String> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn {}: {e}", exe.display()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "missing child stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "missing child stdout".to_string())?;

        let mut player = Self {
            child,
            stdin: BufWriter::new(stdin),
            stdout: BufReader::new(stdout),
        };
        player.send_line("uci")?;
        player.wait_until("uciok")?;
        for (name, value) in options {
            player.send_line(&format!("setoption name {name} value {value}"))?;
        }
        player.send_line("isready")?;
        player.wait_until("readyok")?;
        Ok(player)
    }

    pub fn new_game(&mut self) -> Result<(), String> {
        self.send_line("ucinewgame")?;
        self.send_line("isready")?;
        self.wait_until("readyok")
    }

    pub fn go(&mut self, moves: &[String], spec: SearchSpec) -> Result<MoveSearchResult, String> {
        let pos_cmd = if moves.is_empty() {
            "position startpos".to_string()
        } else {
            format!("position startpos moves {}", moves.join(" "))
        };
        self.send_line(&pos_cmd)?;
        self.send_line(&spec.to_go_command())?;

        let mut result = MoveSearchResult::default();
        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| format!("stdout read error: {e}"))?;
            if read == 0 {
                return Err("engine terminated unexpectedly".to_string());
            }
            let trimmed = line.trim();
            if trimmed.starts_with("info ") {
                parse_info_line(trimmed, &mut result);
            } else if let Some(rest) = trimmed.strip_prefix("bestmove ") {
                let mv = rest.split_whitespace().next().unwrap_or("(none)");
                result.bestmove = (mv != "(none)").then(|| mv.to_string());
                return Ok(result);
            }
        }
    }

    fn send_line(&mut self, line: &str) -> Result<(), String> {
        writeln!(self.stdin, "{line}").map_err(|e| format!("stdin write error: {e}"))?;
        self.stdin
            .flush()
            .map_err(|e| format!("stdin flush error: {e}"))
    }

    fn wait_until(&mut self, token: &str) -> Result<(), String> {
        let mut line = String::new();
        loop {
            line.clear();
            let read = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| format!("stdout read error: {e}"))?;
            if read == 0 {
                return Err(format!("EOF before {token}"));
            }
            if line.trim() == token {
                return Ok(());
            }
        }
    }
}

impl Drop for UciPlayer {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, "quit");
        let _ = self.stdin.flush();
        let _ = self.child.wait();
    }
}

fn parse_info_line(line: &str, out: &mut MoveSearchResult) {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut i = 1usize;
    while i < tokens.len() {
        match tokens[i] {
            "depth" if i + 1 < tokens.len() => {
                if let Ok(v) = tokens[i + 1].parse::<u64>() {
                    out.depth = Some(v);
                }
                i += 2;
            }
            "nodes" if i + 1 < tokens.len() => {
                if let Ok(v) = tokens[i + 1].parse::<u64>() {
                    out.nodes = Some(v);
                }
                i += 2;
            }
            "nps" if i + 1 < tokens.len() => {
                if let Ok(v) = tokens[i + 1].parse::<u64>() {
                    out.nps = Some(v);
                }
                i += 2;
            }
            "score" if i + 2 < tokens.len() => {
                match tokens[i + 1] {
                    "cp" => {
                        if let Ok(v) = tokens[i + 2].parse::<i32>() {
                            out.score_cp = Some(v);
                            out.mate_in = None;
                        }
                    }
                    "mate" => {
                        if let Ok(v) = tokens[i + 2].parse::<i32>() {
                            out.mate_in = Some(v);
                            out.score_cp = Some(if v > 0 { 30_000 } else { -30_000 });
                        }
                    }
                    _ => {}
                }
                i += 3;
            }
            _ => i += 1,
        }
    }
}
