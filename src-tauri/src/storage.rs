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

    pub fn syncs_file(&self) -> PathBuf {
        self.root.join("syncs.json")
    }

    pub fn history_file(&self) -> PathBuf {
        self.root.join("history.jsonl")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// `logs\<yyyyMMdd-HHmmss>-<sync>.log`: the sync part is cut to 60 characters, characters
    /// Windows forbids in file names are replaced and it cannot end in a dot or a space.
    pub fn log_file(&self, sync_name: &str, started: NaiveDateTime) -> PathBuf {
        let safe: String = sync_name
            .chars()
            .take(60)
            .map(|c| {
                if "<>:\"/\\|?*".contains(c) || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let safe = safe.trim_end_matches(['.', ' ']);
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
            storage().syncs_file(),
            Path::new(r"C:\Users\Anna\AppData\Local\ColomboSync\syncs.json")
        );
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

    #[test]
    fn log_file_name_replaces_every_forbidden_character_and_cannot_end_in_a_dot_or_space() {
        let started = NaiveDate::from_ymd_opt(2026, 11, 2)
            .unwrap()
            .and_hms_opt(21, 30, 5)
            .unwrap();
        let log = storage().log_file(r#"a<>:"/\|?*b ."#, started);
        assert_eq!(log.file_name().unwrap(), "20261102-213005-a_________b.log");
    }

    #[test]
    fn a_very_long_sync_name_is_cut_in_the_log_file_name() {
        let started = NaiveDate::from_ymd_opt(2026, 11, 2)
            .unwrap()
            .and_hms_opt(21, 30, 5)
            .unwrap();
        let log = storage().log_file(&"è".repeat(250), started);
        let name = log.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(name, format!("20261102-213005-{}.log", "è".repeat(60)));
    }
}
