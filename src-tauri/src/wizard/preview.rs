//! The Preview of a Sync: a dry run that lists what a Run would do and touches nothing. No Tauri
//! types.

use crate::engine;
use crate::shared::domain::{Preview, SyncMode};
use crate::shared::error_messages::{self, Explanation, Moment};
use crate::shared::{mirror_guard, path_rules};
use std::fs;
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;

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
