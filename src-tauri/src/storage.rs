//! Where ColomboSync keeps its files: `%LOCALAPPDATA%\ColomboSync\`.

use chrono::NaiveDateTime;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    /// Reads the `LOCALAPPDATA` env var (not Tauri's app dir).
    pub fn from_env() -> Result<Self, String> {
        let base = std::env::var_os("LOCALAPPDATA")
            .ok_or("La variabile LOCALAPPDATA non è impostata.".to_string())?;
        Ok(Self::at(Path::new(&base).join("ColomboSync")))
    }

    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn history_file(&self) -> PathBuf {
        self.root.join("history.jsonl")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// `logs\<yyyyMMdd-HHmmss>-<sync>.log`, with characters Windows forbids in file names replaced.
    pub fn log_file(&self, sync_name: &str, started: NaiveDateTime) -> PathBuf {
        let safe: String = sync_name
            .chars()
            .map(|c| {
                if "<>:\"/\\|?*".contains(c) || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        self.logs_dir()
            .join(format!("{}-{}.log", started.format("%Y%m%d-%H%M%S"), safe))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn storage() -> Storage {
        Storage::at(r"C:\Users\Anna\AppData\Local\ColomboSync")
    }

    #[test]
    fn files_live_under_the_root() {
        assert_eq!(
            storage().history_file(),
            Path::new(r"C:\Users\Anna\AppData\Local\ColomboSync\history.jsonl")
        );
        assert_eq!(
            storage().logs_dir(),
            Path::new(r"C:\Users\Anna\AppData\Local\ColomboSync\logs")
        );
    }

    #[test]
    fn log_file_is_named_by_start_time_and_sync_without_forbidden_characters() {
        let started = NaiveDate::from_ymd_opt(2026, 11, 2)
            .unwrap()
            .and_hms_opt(21, 30, 5)
            .unwrap();
        let log = storage().log_file("Copia di Foto: 2024/25", started);
        assert_eq!(
            log.file_name().unwrap(),
            "20261102-213005-Copia di Foto_ 2024_25.log"
        );
        assert_eq!(log.parent().unwrap(), storage().logs_dir());
    }
}
