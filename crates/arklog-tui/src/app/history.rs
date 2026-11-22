use std::path::PathBuf;

use super::TerminalApp;

pub(crate) fn history_path() -> Option<PathBuf> {
    fn variable(name: &str) -> Option<PathBuf> {
        std::env::var_os(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    }
    #[cfg(windows)]
    let root = variable("APPDATA");
    #[cfg(target_os = "macos")]
    let root = variable("HOME").map(|home| home.join("Library/Application Support"));
    #[cfg(not(any(windows, target_os = "macos")))]
    let root =
        variable("XDG_CONFIG_HOME").or_else(|| variable("HOME").map(|home| home.join(".config")));
    let application = if cfg!(any(windows, target_os = "macos")) {
        "ArkLog"
    } else {
        "arklog"
    };
    root.map(|root| root.join(application).join("regex-history.json"))
}

impl TerminalApp {
    pub(crate) fn restore_history(&mut self, path: Option<PathBuf>) {
        self.history_path = path;
        let result = self
            .history_path
            .as_deref()
            .ok_or_else(|| "Regex history unavailable: no user configuration directory".to_string())
            .and_then(|path| {
                self.controller
                    .state_mut()
                    .load_filter_history(path)
                    .map_err(|error| format!("Regex history unavailable: {error}"))
            });
        if result.is_err() {
            self.record(result);
        }
    }

    pub(super) fn persist_history(&self) -> Result<(), String> {
        self.history_path.as_deref().map_or(Ok(()), |path| {
            self.controller
                .state()
                .save_filter_history(path)
                .map_err(|error| format!("Regex history could not be saved: {error}"))
        })
    }
}
