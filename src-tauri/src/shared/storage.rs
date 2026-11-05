//! Where ColomboSync keeps its files: `%LOCALAPPDATA%\ColomboSync\` on Windows,
//! `~/Library/Application Support/ColomboSync` on macOS, `$XDG_DATA_HOME/ColomboSync` or
//! `~/.local/share/ColomboSync` on Linux.

use super::path_rules;
use chrono::NaiveDateTime;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Storage {
    root: PathBuf,
}

impl Storage {
    /// Reads the environment (not Tauri's app dir).
    pub fn from_env() -> Result<Self, String> {
        let env = |name: &str| std::env::var_os(name);
        let root = if cfg!(windows) {
            windows_dir(env)
        } else if cfg!(target_os = "macos") {
            macos_dir(env)
        } else {
            linux_dir(env)
        }?;
        Ok(Self::at(root))
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
                if path_rules::is_forbidden_in_name(c) {
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

/// The bytes of `file`, or `None` when it is not there (yet): a file nobody has written is an
/// empty list, not a failure. Any other read error is reported.
pub fn read_if_present(file: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(file) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

const MISSING_HOME: &str = "La variabile HOME non è impostata.";

fn windows_dir(env: impl Fn(&str) -> Option<OsString>) -> Result<PathBuf, String> {
    let base = env("LOCALAPPDATA").ok_or("La variabile LOCALAPPDATA non è impostata.")?;
    Ok(Path::new(&base).join("ColomboSync"))
}

fn macos_dir(env: impl Fn(&str) -> Option<OsString>) -> Result<PathBuf, String> {
    let home = env("HOME").ok_or(MISSING_HOME)?;
    Ok(Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("ColomboSync"))
}

/// `XDG_DATA_HOME` counts only when it is an absolute path, as the XDG specification says.
fn linux_dir(env: impl Fn(&str) -> Option<OsString>) -> Result<PathBuf, String> {
    if let Some(xdg) = env("XDG_DATA_HOME").map(PathBuf::from)
        && xdg.is_absolute()
    {
        return Ok(xdg.join("ColomboSync"));
    }
    let home = env("HOME").ok_or(MISSING_HOME)?;
    Ok(Path::new(&home)
        .join(".local")
        .join("share")
        .join("ColomboSync"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn storage() -> Storage {
        Storage::at("ColomboSync")
    }

    #[test]
    fn files_live_under_the_root() {
        let root = Path::new("ColomboSync");
        assert_eq!(storage().syncs_file(), root.join("syncs.json"));
        assert_eq!(storage().history_file(), root.join("history.jsonl"));
        assert_eq!(storage().logs_dir(), root.join("logs"));
    }

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn windows_and_macos_have_their_own_data_folder() {
        assert_eq!(
            windows_dir(env(&[("LOCALAPPDATA", "base")])).unwrap(),
            Path::new("base").join("ColomboSync")
        );
        assert!(windows_dir(env(&[])).is_err());

        assert_eq!(
            macos_dir(env(&[("HOME", "/Users/anna")])).unwrap(),
            Path::new("/Users/anna")
                .join("Library")
                .join("Application Support")
                .join("ColomboSync")
        );
        assert!(macos_dir(env(&[])).is_err());
    }

    #[test]
    fn linux_prefers_an_absolute_xdg_data_home_and_falls_back_to_home() {
        let absolute = Path::new(if cfg!(windows) { r"C:\data" } else { "/data" });
        let xdg = absolute.to_str().unwrap();
        assert_eq!(
            linux_dir(env(&[("XDG_DATA_HOME", xdg), ("HOME", "/home/anna")])).unwrap(),
            absolute.join("ColomboSync")
        );
        let fallback = Path::new("/home/anna")
            .join(".local")
            .join("share")
            .join("ColomboSync");
        for xdg in ["dati", ""] {
            assert_eq!(
                linux_dir(env(&[("XDG_DATA_HOME", xdg), ("HOME", "/home/anna")])).unwrap(),
                fallback,
                "XDG_DATA_HOME={xdg:?} is not absolute"
            );
        }
        assert_eq!(linux_dir(env(&[("HOME", "/home/anna")])).unwrap(), fallback);
        assert!(linux_dir(env(&[])).is_err());
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
    fn a_file_that_is_not_there_is_none_and_any_other_read_error_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("x.json");
        assert_eq!(read_if_present(&file).unwrap(), None);

        fs::write(&file, "ciao").unwrap();
        assert_eq!(read_if_present(&file).unwrap(), Some(b"ciao".to_vec()));

        // A folder where the file should be: reading it fails, and it is not "not found".
        assert!(read_if_present(dir.path()).is_err());
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
