//! Zest UCI CLI entry point.

use std::io::{self, BufRead, Write};

use zest::uci::{ParseCommandError, UciCommand, UciSession, parse_command};

fn main() {
    run_uci_loop();
}

fn run_uci_loop() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut session = UciSession::new();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };

        let cmd = match parse_command(&line) {
            Ok(cmd) => cmd,
            Err(ParseCommandError::EmptyLine) => continue,
            Err(err) => {
                let _ = writeln!(out, "info string {err}");
                let _ = out.flush();
                continue;
            }
        };

        if matches!(cmd, UciCommand::Quit) {
            break;
        }

        for response in session.handle_command_sync(&cmd) {
            let _ = writeln!(out, "{response}");
        }
        let _ = out.flush();
    }
}
