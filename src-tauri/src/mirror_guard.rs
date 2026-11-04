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

/// What macOS and Linux put in the root of a disk, with the case macOS writes them in.
const UNIX_SYSTEM_ENTRIES: [&str; 6] = [
    ".DS_Store",
    ".Spotlight-V100",
    ".fseventsd",
    ".Trashes",
    ".TemporaryItems",
    "lost+found",
];

/// `.Trash-1000` is the trash a Linux desktop keeps on each disk, one per user id.
const LINUX_TRASH_PREFIX: &str = ".Trash-";

/// Names compare without regard to case on Windows and macOS, exactly on Linux.
fn is_unix_system_entry(name: &str, ignore_case: bool) -> bool {
    if ignore_case {
        let lower = name.to_lowercase();
        UNIX_SYSTEM_ENTRIES
            .iter()
            .any(|known| known.to_lowercase() == lower)
            || lower.starts_with(&LINUX_TRASH_PREFIX.to_lowercase())
    } else {
        UNIX_SYSTEM_ENTRIES.contains(&name) || name.starts_with(LINUX_TRASH_PREFIX)
    }
}

fn is_system_entry(name: &str) -> bool {
    SYSTEM_ENTRIES.contains(&name.to_lowercase().as_str())
        || is_unix_system_entry(name, !cfg!(target_os = "linux"))
}

/// A is missing or has no entries (system entries do not count).
pub fn source_problem(source: &Path) -> Option<&'static str> {
    let has_entries = fs::read_dir(source).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| !is_system_entry(&entry.file_name().to_string_lossy()))
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
    fn unix_system_entries_do_not_make_a_source_non_empty() {
        let temp = tempfile::tempdir().unwrap();
        for dir in [
            ".Spotlight-V100",
            ".fseventsd",
            ".Trashes",
            ".TemporaryItems",
            "lost+found",
            ".Trash-1000",
        ] {
            fs::create_dir(temp.path().join(dir)).unwrap();
        }
        fs::write(temp.path().join(".DS_Store"), "x").unwrap();

        assert_eq!(source_problem(temp.path()), Some(SOURCE_EMPTY));

        fs::write(temp.path().join("foto.jpg"), "x").unwrap();
        assert_eq!(source_problem(temp.path()), None);
    }

    #[test]
    fn unix_system_names_are_exact_on_linux_and_any_case_elsewhere() {
        for (name, exact, any_case) in [
            (".DS_Store", true, true),
            (".ds_store", false, true),
            (".SPOTLIGHT-V100", false, true),
            (".Trash-1000", true, true),
            (".trash-1000", false, true),
            ("lost+found", true, true),
            ("Lost+Found", false, true),
            (".Trashes", true, true),
            (".Trash", false, false),
            ("foto.jpg", false, false),
            (".DS_Store.bak", false, false),
        ] {
            assert_eq!(is_unix_system_entry(name, false), exact, "{name} exact");
            assert_eq!(
                is_unix_system_entry(name, true),
                any_case,
                "{name} any case"
            );
        }
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
