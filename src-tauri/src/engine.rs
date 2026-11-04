//! The copy engine: robocopy on Windows, rsync on macOS and Linux. The rest of the app (runner,
//! history, MirrorGuard, UI) talks to this module only and never learns which one is underneath.
//! Each engine module exposes the same functions; `cfg(target_os)` picks one at compile time.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMode {
    Copy,
    Mirror,
}

/// A dry run that has ended.
pub struct Finished {
    /// Exit code of the engine process.
    pub code: i32,
    /// What the engine wrote, as text: robocopy's `/L` log, or rsync's stdout and stderr.
    pub log: String,
}

#[cfg(windows)]
pub use crate::robocopy::{
    decode_log, dry_run, exit_code, find_cause, is_success, parse_preview, partial_count, spawn,
};
#[cfg(unix)]
pub use crate::rsync::{
    decode_log, dry_run, exit_code, find_cause, is_success, parse_preview, partial_count, spawn,
};
