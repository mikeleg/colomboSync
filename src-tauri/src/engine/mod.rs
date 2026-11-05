//! The copy engine: robocopy on Windows, rsync on macOS and Linux. The rest of the app (runner,
//! history, MirrorGuard, UI) talks to this module only and never learns which one is underneath.
//! Each engine module exposes the same functions; `cfg(target_os)` picks one at compile time.

pub mod preview_parser;
pub mod robocopy;
pub mod rsync;

use crate::shared::domain::{Cause, Folder};
use crate::shared::error_messages::ENGINE_MISSING;

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
pub use robocopy::{
    decode_log, dry_run, exit_code, find_cause, is_success, parse_preview, partial_count, spawn,
};
#[cfg(unix)]
pub use rsync::{
    decode_log, dry_run, exit_code, find_cause, is_success, parse_preview, partial_count, spawn,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::error_messages::{self, Moment};
    use std::io;

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
