//! The copy engine: robocopy on Windows, rsync on macOS and Linux. The rest of the app (runner,
//! history, MirrorGuard, UI) talks to this module only and never learns which one is underneath.
//! Each engine module exposes the same functions; `cfg(target_os)` picks one at compile time.

pub mod preview_parser;
pub mod robocopy;
pub mod rsync;

use crate::shared::domain::{Cause, Folder};
use crate::shared::error_messages::ENGINE_MISSING;
use std::fs;
use std::path::Path;

/// A dry run that has ended.
pub struct Finished {
    /// Exit code of the engine process.
    pub code: i32,
    /// What the engine wrote, as text: robocopy's `/L` log, or rsync's stdout and stderr.
    pub log: String,
}

/// The cause to record when the engine program is not there (rsync is not installed; robocopy is
/// part of Windows); nothing else about a failed start is worth a message of its own.
pub fn missing_engine(error: &std::io::Error) -> Option<Cause> {
    (cfg!(unix) && error.kind() == std::io::ErrorKind::NotFound).then_some(Cause {
        code: ENGINE_MISSING,
        folder: Folder::A,
    })
}

#[cfg(windows)]
use robocopy::partial_count;
#[cfg(windows)]
pub use robocopy::{
    decode_log, dry_run, exit_code, file_name, find_cause, is_file_line, is_success, parse_preview,
    spawn,
};
#[cfg(unix)]
use rsync::partial_count;
#[cfg(unix)]
pub use rsync::{
    decode_log, dry_run, exit_code, file_name, find_cause, is_file_line, is_success, parse_preview,
    spawn,
};

/// How a Run ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    /// Stopped by hand, after `partial` files.
    Cancelled {
        partial: usize,
    },
    /// Failed after `partial` files; `cause` is `None` when the engine gave no reason worth a
    /// message of its own.
    Error {
        cause: Option<Cause>,
        partial: usize,
    },
}

/// How a Run ended, from the engine's exit `code`, whether the user stopped it (`cancelled`) and the
/// log it left. A stop is told apart here: a killed robocopy exits with code 1, which alone would
/// read as success, and a killed rsync has no exit code of its own (128 + 9). Only a failed or
/// stopped run needs its log (it can be huge), so only then is the file read.
pub fn verdict(
    code: i32,
    cancelled: bool,
    log_file: &Path,
    source: &str,
    destination: &str,
) -> Verdict {
    judge(code, cancelled, || read_log(log_file), source, destination)
}

fn judge(
    code: i32,
    cancelled: bool,
    read_log: impl FnOnce() -> String,
    source: &str,
    destination: &str,
) -> Verdict {
    if !cancelled && is_success(code) {
        return Verdict::Ok;
    }
    let log = read_log();
    let partial = partial_count(&log, source, destination, code, cancelled);
    if cancelled {
        Verdict::Cancelled { partial }
    } else {
        Verdict::Error {
            cause: find_cause(code, &log, source, destination),
            partial,
        }
    }
}

/// A run's log file as text; empty when it cannot be read.
pub fn read_log(file: &Path) -> String {
    fs::read(file)
        .map(|bytes| decode_log(&bytes))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::error_messages::{self, Moment};
    use std::io;

    #[test]
    fn a_run_that_ends_well_is_ok_and_its_log_is_never_read() {
        let verdict = judge(
            0,
            false,
            || panic!("the log of an Ok run was read"),
            "A",
            "B",
        );
        assert_eq!(verdict, Verdict::Ok);
    }

    #[test]
    fn a_stop_wins_over_any_exit_code() {
        // A killed robocopy exits with 1 (a success code), a killed rsync with 137.
        for code in [0, 1, 137] {
            let verdict = judge(code, true, String::new, "A", "B");
            assert_eq!(verdict, Verdict::Cancelled { partial: 0 }, "exit {code}");
        }
    }

    #[test]
    fn a_failure_without_a_known_reason_has_no_cause() {
        let verdict = judge(99, false, String::new, "A", "B");
        assert_eq!(
            verdict,
            Verdict::Error {
                cause: None,
                partial: 0
            }
        );
    }

    #[test]
    fn the_log_of_a_failed_run_is_read_from_its_file_and_a_missing_file_reads_empty() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("non-c-e.log");
        assert_eq!(read_log(&missing), "");
        assert_eq!(
            verdict(99, false, &missing, "A", "B"),
            Verdict::Error {
                cause: None,
                partial: 0
            }
        );
    }

    #[test]
    fn a_program_that_is_not_found_is_a_missing_engine_only_where_the_engine_is_installed_by_hand()
    {
        let not_found = io::Error::from(io::ErrorKind::NotFound);
        assert_eq!(missing_engine(&not_found).is_some(), cfg!(unix));
        assert_eq!(
            missing_engine(&io::Error::from(io::ErrorKind::PermissionDenied)),
            None
        );
        if cfg!(unix) {
            let explanation =
                error_messages::describe(missing_engine(&not_found), "/a", "/b", Moment::Preview);
            assert_eq!(explanation.title, "Su questo computer manca rsync.");
        }
    }
}
