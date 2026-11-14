use crate::uci::command::UciCommand;

/// Stateful UCI session holding engine components across commands.
#[derive(Debug, Default)]
pub struct UciSession;

impl UciSession {
    /// Creates a new [`UciSession`].
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Executes a synchronous [`UciCommand`] and returns lines to write to stdout.
    #[must_use]
    pub fn handle_command_sync(&mut self, cmd: &UciCommand) -> Vec<String> {
        match cmd {
            UciCommand::Uci => vec![
                "id name Zest".to_string(),
                "id author niwatoriiiiiiiii".to_string(),
                "option name UCI_Variant type combo default gardner var gardner".to_string(),
                "uciok".to_string(),
            ],
            UciCommand::IsReady => vec!["readyok".to_string()],
            UciCommand::UciNewGame | UciCommand::SetOption { .. } | UciCommand::Quit => Vec::new(),
        }
    }
}
