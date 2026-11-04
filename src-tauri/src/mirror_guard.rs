//! MirrorGuard: stops a mirror run that would wipe B because A is unplugged or empty.
//! Cheap on purpose: one `read_dir`, no walk of the tree.

use crate::preview_parser::Counts;
use std::fs;
use std::path::Path;

pub const WARNING: &str = "A sembra vuota o scollegata: con lo specchio verrebbe svuotato B. Controlla che il disco di A sia collegato.";

/// True when a mirror run must not start: A is missing or has no entries, or the preview would
/// delete more files than it copies or updates.
pub fn blocks(source: &Path, counts: Counts) -> bool {
    !has_entries(source) || counts.deleted > counts.new + counts.updated
}

fn has_entries(folder: &Path) -> bool {
    fs::read_dir(folder).is_ok_and(|mut entries| entries.next().is_some())
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

        assert!(blocks(&temp.path().join("non-esiste"), busy));
        assert!(blocks(temp.path(), busy), "empty folder");

        fs::write(temp.path().join("a.txt"), "x").unwrap();
        assert!(!blocks(temp.path(), busy));
    }

    #[test]
    fn blocks_when_deleted_exceed_new_plus_updated() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("a.txt"), "x").unwrap();

        assert!(blocks(temp.path(), counts(1, 1, 3)));
        assert!(blocks(temp.path(), counts(0, 0, 1)));
        assert!(!blocks(temp.path(), counts(1, 1, 2)), "equal is allowed");
        assert!(!blocks(temp.path(), counts(0, 0, 0)));
    }
}
