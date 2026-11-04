//! Turns a decoded `/L` log into a `Preview`. Lines are classified by full path, never by the
//! (localized) robocopy labels.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Most files listed per group; the rest is only counted (trees can have 100k+ files).
pub const MAX_LISTED: usize = 200;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileGroup {
    pub count: usize,
    /// First `MAX_LISTED` paths, relative to A (new, updated) or to B (deleted).
    pub files: Vec<String>,
}

impl FileGroup {
    fn add(&mut self, relative: &str) {
        self.count += 1;
        if self.files.len() < MAX_LISTED {
            self.files.push(relative.to_string());
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preview {
    pub new: FileGroup,
    pub updated: FileGroup,
    pub deleted: FileGroup,
    /// Size in bytes of what would be copied (new + updated).
    pub bytes: u64,
    /// Why this mirror run must not start (MirrorGuard), in Italian; `None` when it may.
    pub blocked: Option<String>,
}

/// How many files a run changed; what the UI sends back from the preview it just showed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    pub new: usize,
    pub updated: usize,
    pub deleted: usize,
}

impl Preview {
    pub fn counts(&self) -> Counts {
        Counts {
            new: self.new.count,
            updated: self.updated.count,
            deleted: self.deleted.count,
        }
    }
}

/// `exists` tells whether a path exists in B; it separates Updated from New.
pub fn parse(
    log: &str,
    source: &str,
    destination: &str,
    exists: impl Fn(&Path) -> bool,
) -> Preview {
    let source_prefix = prefix_of(source);
    let destination_prefix = prefix_of(destination);
    let mut preview = Preview::default();

    for line in log.lines() {
        let Some((size, path)) = size_and_path(line, &[&source_prefix, &destination_prefix]) else {
            continue;
        };
        if is_in_destination(path, &source_prefix, &destination_prefix) {
            preview.deleted.add(&path[destination_prefix.len()..]);
        } else {
            let relative = &path[source_prefix.len()..];
            let in_destination = Path::new(&destination_prefix).join(relative);
            if exists(&in_destination) {
                preview.updated.add(relative);
            } else {
                preview.new.add(relative);
            }
            preview.bytes += size;
        }
    }
    preview
}

/// Files of A that a run's own log lists. Unlike `parse` it does not classify: after a copy every
/// file exists in B, so "Updated" would be meaningless.
pub fn count_source_files(log: &str, source: &str, destination: &str) -> usize {
    let source_prefix = prefix_of(source);
    let destination_prefix = prefix_of(destination);
    log.lines()
        .filter_map(|line| size_and_path(line, &[&source_prefix, &destination_prefix]))
        .filter(|(_, path)| !is_in_destination(path, &source_prefix, &destination_prefix))
        .count()
}

/// Path under B rather than A; with B inside A (or A inside B) the longer root wins.
fn is_in_destination(path: &str, source_prefix: &str, destination_prefix: &str) -> bool {
    starts_with_ci(path, destination_prefix)
        && (!starts_with_ci(path, source_prefix) || destination_prefix.len() > source_prefix.len())
}

/// Root with `\` separators and exactly one trailing `\`, so `D:\Foto` never matches `D:\Foto2`.
fn prefix_of(root: &str) -> String {
    let mut prefix = root.trim().replace('/', "\\");
    while prefix.ends_with('\\') {
        prefix.pop();
    }
    prefix.push('\\');
    prefix
}

fn starts_with_ci(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// A file line is `[label] <size> <full path>`: the path starts at the first root found after
/// whitespace and the token right before it is the size in bytes. Anything else (blank lines,
/// error messages that merely mention a path) is not a file line.
fn size_and_path<'a>(line: &'a str, prefixes: &[&str]) -> Option<(u64, &'a str)> {
    let start = line
        .char_indices()
        .filter(|&(i, _)| i == 0 || line[..i].ends_with(char::is_whitespace))
        .find(|&(i, _)| prefixes.iter().any(|p| starts_with_ci(&line[i..], p)))?
        .0;
    let size = line[..start].split_whitespace().last()?.parse().ok()?;
    Some((size, line[start..].trim_end()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::robocopy::decode_unilog;

    // Real `/E /L /NJH /NJS /NDL /NC /BYTES /FP /NP /UNILOG` output (UTF-16LE with BOM) from
    // temp folders, paths rewritten to D:\Foto and E:\Backup, plus one hand-added line carrying
    // an Italian label ("Nuovo file").
    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/preview_unilog_utf16le.txt");

    #[test]
    fn classifies_new_updated_and_extra_by_path() {
        let log = decode_unilog(FIXTURE);
        let exists = |p: &Path| p == Path::new(r"E:\Backup\updated.txt");

        let preview = parse(&log, r"D:\Foto", r"E:\Backup\", exists);

        assert_eq!(
            preview.new.files,
            [
                "new.txt",
                "è già qui.txt",
                r"sub\deep.txt",
                r"sub\con etichetta.txt"
            ]
        );
        assert_eq!(preview.updated.files, ["updated.txt"]);
        assert_eq!(
            preview.deleted.files,
            ["only-in-b.txt", r"sub\only-in-b-too.txt"]
        );
        assert_eq!(
            preview.counts(),
            Counts {
                new: 4,
                updated: 1,
                deleted: 2
            }
        );
        assert_eq!(preview.bytes, 7 + 22 + 12 + 3 + 15);
    }

    #[test]
    fn paths_are_compared_case_insensitively_and_unc_is_supported() {
        let log = "\t\t 5\t\\\\NAS\\Share\\Foto\\a.txt\r\n\t\t 6\t\\\\nas\\backup\\x\\b.txt\r\n";
        let preview = parse(log, r"\\nas\share\foto\", r"\\NAS\Backup\x", |_| false);
        assert_eq!(preview.new.files, ["a.txt"]);
        assert_eq!(preview.deleted.files, ["b.txt"]);
    }

    #[test]
    fn lines_that_are_not_files_are_ignored() {
        let log = "\r\n2026/11/03 12:00:00 ERROR 5 (0x00000005) Copying File D:\\Foto\\a.txt\r\nAccess is denied.\r\n";
        let preview = parse(log, r"D:\Foto", r"E:\Backup", |_| false);
        assert_eq!(preview, Preview::default());
    }

    #[test]
    fn counts_file_lines_in_run_log() {
        let log = "\u{feff}\r\n\t  \t\t 5\tD:\\Foto\\a.txt\r\n\t  \t\t 6\td:\\foto\\sub\\è.txt\r\n2026/11/03 12:00:00 ERROR 5 (0x00000005) Copying File D:\\Foto\\b.txt\r\nAccess is denied.\r\n\t  \t\t 7\tE:\\Backup\\extra.txt\r\n";
        assert_eq!(count_source_files(log, r"D:\Foto", r"E:\Backup"), 2);
        assert_eq!(count_source_files("", r"D:\Foto", r"E:\Backup"), 0);
    }

    #[test]
    fn lines_under_b_are_not_counted_when_b_is_inside_a() {
        let log = "\t\t 5\tD:\\Foto\\a.txt\r\n\t\t 6\tD:\\Foto\\Backup\\a.txt\r\n\t\t 7\tD:\\Foto\\Backup\\sub\\b.txt\r\n";
        assert_eq!(count_source_files(log, r"D:\Foto", r"D:\Foto\Backup"), 1);
    }

    #[test]
    fn long_lists_are_capped_but_counted() {
        let log: String = (0..MAX_LISTED + 5)
            .map(|i| format!("\t\t 1\tD:\\Foto\\f{i}.txt\r\n"))
            .collect();
        let preview = parse(&log, r"D:\Foto", r"E:\Backup", |_| false);
        assert_eq!(preview.new.count, MAX_LISTED + 5);
        assert_eq!(preview.new.files.len(), MAX_LISTED);
    }
}
