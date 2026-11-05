//! The Preview of a Sync: a dry run that lists what a Run would do and touches nothing. No Tauri
//! types.

use crate::engine;
use crate::engine::preview_parser::{Counts, Preview};
use crate::shared::domain::SyncMode;
use crate::shared::error_messages::{self, Explanation, Moment};
use crate::shared::{mirror_guard, path_rules};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Dry run: lists what a Run would do, touches nothing. Blocks until the engine exits.
pub fn preview(source: &str, destination: &str, mode: SyncMode) -> Result<Preview, Explanation> {
    path_rules::check_destination(source, destination)?;

    // A that cannot be read says why (unplugged disk, share down, no permission): that is not
    // the same as an A with nothing in it, and the engine would only fail on it.
    fs::metadata(source)
        .map_err(|error| error_messages::explain_source_error(&error, source, destination))?;
    // An empty A would empty B: answer "blocked" without running the engine.
    if mode == SyncMode::Mirror
        && let Some(reason) = mirror_guard::source_problem(Path::new(source))
    {
        return Ok(Preview {
            blocked: Some(reason.to_string()),
            ..Preview::default()
        });
    }

    let finished = engine::dry_run(mode, source, destination).map_err(|error| {
        // Never the raw OS error: either the engine is not installed or it is a generic failure.
        error_messages::describe(
            engine::missing_engine(&error),
            source,
            destination,
            Moment::Preview,
        )
    })?;

    if !engine::is_success(finished.code) {
        let cause = engine::find_cause(finished.code, &finished.log, source, destination);
        return Err(error_messages::describe(
            cause,
            source,
            destination,
            Moment::Preview,
        ));
    }
    let mut preview = engine::parse_preview(&finished.log, source, destination);
    if mode == SyncMode::Mirror {
        preview.blocked = mirror_guard::counts_problem(preview.counts()).map(str::to_string);
    }
    Ok(preview)
}

pub const NO_PREVIEW: &str = "Prima controlla le due cartelle con l’anteprima, poi avvia.";

type LastPreview = Option<(String, String, SyncMode, Counts)>;

/// The last preview shown to the user. A Mirror run takes its counts from here, never from what
/// the UI sends back, and is refused when no preview of the same A, B and mode came first.
#[derive(Default)]
pub struct PreviewMemo(Mutex<LastPreview>);

impl PreviewMemo {
    pub fn remember(&self, source: &str, destination: &str, mode: SyncMode, counts: Counts) {
        *self.slot() = Some((source.into(), destination.into(), mode, counts));
    }

    /// Counts of the preview this run confirms; each preview confirms one run. Copy never
    /// deletes, so without a matching preview it falls back to the `sent` counts.
    pub fn take_counts(
        &self,
        source: &str,
        destination: &str,
        mode: SyncMode,
        sent: Counts,
    ) -> Result<Counts, String> {
        match self.slot().take() {
            Some((s, d, m, counts)) if s == source && d == destination && m == mode => Ok(counts),
            _ if mode == SyncMode::Copy => Ok(sent),
            _ => Err(NO_PREVIEW.to_string()),
        }
    }

    fn slot(&self) -> MutexGuard<'_, LastPreview> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(new: usize, updated: usize, deleted: usize) -> Counts {
        Counts {
            new,
            updated,
            deleted,
        }
    }

    #[test]
    fn a_mirror_run_takes_its_counts_from_the_matching_preview_once() {
        let memo = PreviewMemo::default();
        let sent = counts(9, 9, 0);
        let (a, b) = (r"D:\Foto", r"E:\Backup");

        assert_eq!(
            memo.take_counts(a, b, SyncMode::Mirror, sent),
            Err(NO_PREVIEW.to_string()),
            "no preview at all"
        );

        memo.remember(a, b, SyncMode::Mirror, counts(1, 0, 5));
        assert_eq!(
            memo.take_counts(a, b, SyncMode::Mirror, sent),
            Ok(counts(1, 0, 5)),
            "what the UI sends is ignored"
        );
        assert!(
            memo.take_counts(a, b, SyncMode::Mirror, sent).is_err(),
            "one preview confirms one run"
        );

        for (source, destination, mode) in [
            (r"D:\Altro", b, SyncMode::Mirror),
            (a, r"E:\Altro", SyncMode::Mirror),
            (a, b, SyncMode::Copy),
        ] {
            memo.remember(a, b, SyncMode::Mirror, counts(1, 0, 0));
            let taken = memo.take_counts(source, destination, mode, sent);
            assert_eq!(
                taken.is_err(),
                mode == SyncMode::Mirror,
                "{source} {destination}"
            );
        }
    }

    #[test]
    fn a_copy_run_without_a_matching_preview_uses_the_sent_counts() {
        let memo = PreviewMemo::default();
        assert_eq!(
            memo.take_counts("A", "B", SyncMode::Copy, counts(2, 1, 0)),
            Ok(counts(2, 1, 0))
        );
    }

    #[test]
    fn preview_enforces_the_path_rules_itself() {
        for (a, b, message) in path_rules::refused_pairs() {
            for mode in [SyncMode::Copy, SyncMode::Mirror] {
                assert_eq!(preview(a, b, mode).unwrap_err(), Explanation::from(message));
            }
        }
    }

    #[test]
    fn preview_of_a_missing_source_says_where_to_look_in_both_modes() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("Foto");
        let missing = missing.to_str().unwrap();
        let backup = temp.path().join("Backup");
        let backup = backup.to_str().unwrap();

        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let failure = preview(missing, backup, mode).unwrap_err();
            assert_eq!(
                failure.title,
                format!("Non trovo la cartella A ({missing}).")
            );
            assert_eq!(failure.advice, "Il disco è collegato?");
        }
    }
}
