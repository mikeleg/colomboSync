//! MirrorGuard: stops a mirror run that would wipe B because A is unplugged or empty.
//! Cheap on purpose: one `read_dir`, no walk of the tree. Each check answers with the Italian
//! reason to show, or `None` when the mirror may go on.

use crate::preview_parser::Counts;
use std::fs;
use std::path::Path;

pub const SOURCE_EMPTY: &str = "A sembra vuota o scollegata: con lo specchio verrebbe svuotato B. Controlla che il disco di A sia collegato.";
pub const TOO_MANY_DELETES: &str = "Con lo specchio verrebbero cancellati da B più file di quelli che arrivano da A. Controlla di aver scelto le cartelle giuste.";

/// What Windows puts in the root of a drive: not a sign that A holds anything of the user's.
const SYSTEM_ENTRIES: [&str; 4] = [
    "system volume information",
    "$recycle.bin",
    "desktop.ini",
    "thumbs.db",
];

/// A is missing or has no entries (system entries do not count).
pub fn source_problem(source: &Path) -> Option<&'static str> {
    let has_entries = fs::read_dir(source).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            !SYSTEM_ENTRIES.contains(&name.as_str())
        })
    });
    (!has_entries).then_some(SOURCE_EMPTY)
}

/// The preview would delete more files than it copies or updates.
pub fn counts_problem(counts: Counts) -> Option<&'static str> {
    (counts.deleted > counts.new + counts.updated).then_some(TOO_MANY_DELETES)
}

/// Both checks, for the moment a run is about to start.
pub fn problem(source: &Path, counts: Counts) -> Option<&'static str> {
    source_problem(source).or_else(|| counts_problem(counts))
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
    fn blocks_when_source_missing_or_empty() {
        let temp = tempfile::tempdir().unwrap();
        let busy = counts(5, 0, 0);

        assert_eq!(
            problem(&temp.path().join("non-esiste"), busy),
            Some(SOURCE_EMPTY)
        );
        assert_eq!(problem(temp.path(), busy), Some(SOURCE_EMPTY), "empty");

        fs::write(temp.path().join("a.txt"), "x").unwrap();
        assert_eq!(problem(temp.path(), busy), None);
    }

    #[test]
    fn system_entries_do_not_make_a_source_non_empty() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("System Volume Information")).unwrap();
        fs::create_dir(temp.path().join("$RECYCLE.BIN")).unwrap();
        fs::write(temp.path().join("Desktop.ini"), "x").unwrap();
        fs::write(temp.path().join("thumbs.db"), "x").unwrap();

        assert_eq!(source_problem(temp.path()), Some(SOURCE_EMPTY));

        fs::write(temp.path().join("foto.jpg"), "x").unwrap();
        assert_eq!(source_problem(temp.path()), None);
    }

    #[test]
    fn blocks_when_deleted_exceed_new_plus_updated() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("a.txt"), "x").unwrap();

        assert_eq!(
            problem(temp.path(), counts(1, 1, 3)),
            Some(TOO_MANY_DELETES)
        );
        assert_eq!(
            problem(temp.path(), counts(0, 0, 1)),
            Some(TOO_MANY_DELETES)
        );
        assert_eq!(problem(temp.path(), counts(1, 1, 2)), None, "equal is ok");
        assert_eq!(problem(temp.path(), counts(0, 0, 0)), None);
    }
}
