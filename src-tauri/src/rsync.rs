//! rsync engine (macOS and Linux): argument builder, itemize-line parser, exit-code rules and
//! the process calls. The text rules are plain functions, so they are tested on every platform;
//! only the process calls are Unix-only. Facts and per-platform quirks: `docs/agents/rsync.md`.

use crate::engine::SyncMode;
use crate::error_messages::{Cause, Folder};
use crate::preview_parser::Preview;
use std::path::Path;

/// `-rt`: recurse and keep modification times (so the next run finds nothing to do); no
/// permissions, owners or links, which external disks (exFAT, NTFS) cannot store.
/// `-8`: file names stay UTF-8 instead of `\#303\#250` escapes in a non-UTF-8 locale.
/// `--itemize-changes`: one `<code> <name>` line per file, read by code, never by label.
const BASE_FLAGS: [&str; 3] = ["-rt", "-8", "--itemize-changes"];

/// Arguments of the `--dry-run` that lists what a run would do.
pub fn preview_args(mode: SyncMode, source: &str, destination: &str) -> Vec<String> {
    args(mode, source, destination, true)
}

/// Arguments of the real run. Its stdout is the same itemize list as the preview's.
pub fn run_args(mode: SyncMode, source: &str, destination: &str) -> Vec<String> {
    args(mode, source, destination, false)
}

fn args(mode: SyncMode, source: &str, destination: &str, dry_run: bool) -> Vec<String> {
    let mut args: Vec<String> = BASE_FLAGS.iter().map(|flag| flag.to_string()).collect();
    if mode == SyncMode::Mirror {
        args.push("--delete".into());
    }
    if dry_run {
        args.push("--dry-run".into());
    }
    // The trailing `/` copies what is inside A, not A itself.
    args.push(with_trailing_slash(source));
    args.push(with_trailing_slash(destination));
    args
}

fn with_trailing_slash(path: &str) -> String {
    let path = path.trim();
    if path.ends_with('/') {
        path.to_string()
    } else {
        format!("{path}/")
    }
}

/// Exit 0 is the only success. 24 ("some files vanished") and 23 ("some files could not be
/// transferred") are failures: the user must hear that the copy is not complete.
pub fn is_success(exit_code: i32) -> bool {
    exit_code == 0
}

/// What one itemize line says about one file.
enum Item<'a> {
    /// `>f…`: the file is copied from A to B; the code says whether it is new.
    Copied { code: &'a str, name: &'a str },
    /// `*deleting`: a file of B that a mirror removes.
    Deleted { name: &'a str },
}

/// Files only: directories (names ending in `/`), links and rsync's own notices
/// (`skipping non-regular file`, `created directory`) are not items.
fn item(line: &str) -> Option<Item<'_>> {
    let line = line.trim_end_matches('\r');
    let (code, rest) = line.split_once(' ')?;
    if code == "*deleting" {
        // GNU rsync pads the word with spaces, openrsync does not.
        let name = rest.trim_start();
        return (!name.is_empty() && !name.ends_with('/')).then_some(Item::Deleted { name });
    }
    // The code is `>f` plus attribute letters; its length differs (11 characters on GNU rsync,
    // 9 on openrsync), so only the start is read. One space separates it from the name.
    (code.starts_with(">f") && !rest.is_empty() && !rest.ends_with('/'))
        .then_some(Item::Copied { code, name: rest })
}

/// `destination_exists` is `false` when B is not there yet: openrsync's dry run then prints no
/// `+` flags, so every copied file is New. `size_of` gives the size of a file of A, by its path
/// relative to A (itemize lines carry no sizes).
pub fn parse(log: &str, destination_exists: bool, size_of: impl Fn(&str) -> u64) -> Preview {
    let mut preview = Preview::default();
    for line in log.lines() {
        match item(line) {
            Some(Item::Copied { code, name }) => {
                if destination_exists && !code.contains('+') {
                    preview.updated.add(name);
                } else {
                    preview.new.add(name);
                }
                preview.bytes += size_of(name);
            }
            Some(Item::Deleted { name }) => preview.deleted.add(name),
            None => {}
        }
    }
    preview
}

/// The preview of a dry run's log; sizes and the existence of B are read from the disk.
pub fn parse_preview(log: &str, source: &str, destination: &str) -> Preview {
    let source = Path::new(source.trim());
    parse(log, Path::new(destination.trim()).is_dir(), |name| {
        std::fs::metadata(source.join(name)).map_or(0, |meta| meta.len())
    })
}

/// Files copied before a failed or stopped run ended. The log lists a file when rsync starts on
/// it, so the last one is the file in flight (possibly cut short) and is not counted.
pub fn partial_count(log: &str, _source: &str, _destination: &str) -> usize {
    let copied = log
        .lines()
        .filter(|line| matches!(item(line), Some(Item::Copied { .. })))
        .count();
    copied.saturating_sub(1)
}

/// A run's log is plain UTF-8 (rsync output as the app captured it).
pub fn decode_log(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Is this stdout line a file the run has just started on (or deleted)? Used by the progress.
pub fn is_file_line(line: &str) -> bool {
    item(line).is_some()
}

/// File name (no folders) of an itemize line.
pub fn file_name(line: &str) -> Option<&str> {
    let name = match item(line)? {
        Item::Copied { name, .. } | Item::Deleted { name } => name,
    };
    name.rsplit('/').next()
}

/// The reason of a failed rsync, in the Windows-style codes the messages of `error_messages`
/// are keyed on (2/3 not found, 5 no permission, 53 unreachable). It is read from the exit
/// code and from what the two folders are like right now, never from rsync's English text.
/// Exit 23 and 11 cover many things, so they are only explained when a folder is missing or
/// cannot be opened; every other failure stays generic ("apri il registro"). `_log` is there
/// because robocopy's cause is in its log; rsync's text is never read.
pub fn find_cause(exit_code: i32, _log: &str, source: &str, destination: &str) -> Option<Cause> {
    let folders = [(Folder::A, source), (Folder::B, destination)];
    let missing = folders
        .iter()
        .find(|(_, path)| !Path::new(path.trim()).exists())
        .map(|(folder, _)| *folder);
    match exit_code {
        // Socket and stream errors, timeouts: the other side went away.
        10 | 12 | 30 | 35 => Some(Cause {
            code: 53,
            folder: missing.unwrap_or(Folder::B),
        }),
        3 | 11 | 23 => match missing {
            Some(folder) => Some(Cause { code: 3, folder }),
            None => folders
                .iter()
                .find(|(_, path)| {
                    std::fs::read_dir(path.trim())
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied)
                })
                .map(|(folder, _)| Cause {
                    code: 5,
                    folder: *folder,
                }),
        },
        _ => None,
    }
}

#[cfg(unix)]
pub use process::{dry_run, exit_code, spawn};

#[cfg(unix)]
mod process {
    use super::*;
    use crate::engine::Finished;
    use std::fs::File;
    use std::io::{self, PipeReader, Read, Write};
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Child, Command, ExitStatus, Stdio};

    /// rsync exit code for "error in file IO": what GNU rsync says when B cannot be made.
    const FILE_IO_ERROR: i32 = 11;

    fn command(args: &[String]) -> Command {
        let mut command = Command::new("rsync");
        command.args(args).stdin(Stdio::null());
        command
    }

    /// openrsync creates every missing folder above B, which would fill the computer's own
    /// disk when B is on an unplugged one (`/Volumes/Backup/Foto`). Only B itself may be new.
    fn parent_is_missing(destination: &str) -> bool {
        let destination = Path::new(destination.trim());
        !destination.exists()
            && destination
                .parent()
                .is_some_and(|parent| !parent.as_os_str().is_empty() && !parent.exists())
    }

    /// Runs the dry run and blocks until it exits. `log` holds stdout then stderr.
    pub fn dry_run(mode: SyncMode, source: &str, destination: &str) -> io::Result<Finished> {
        if parent_is_missing(destination) {
            return Ok(Finished {
                code: FILE_IO_ERROR,
                log: String::new(),
            });
        }
        let output = command(&preview_args(mode, source, destination)).output()?;
        let mut log = String::from_utf8_lossy(&output.stdout).into_owned();
        log.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(Finished {
            code: exit_code(output.status)?,
            log,
        })
    }

    /// Starts the real run. stdout and stderr share one pipe; what is read from it is also
    /// written to `log`, which is the run's log for the history.
    pub fn spawn(
        mode: SyncMode,
        source: &str,
        destination: &str,
        log: &Path,
    ) -> io::Result<(Child, Box<dyn Read + Send>)> {
        if parent_is_missing(destination) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "la cartella B non esiste",
            ));
        }
        let log = File::create(log)?;
        let (reader, writer) = io::pipe()?;
        let child = {
            // The command holds the write end of the pipe: it is dropped before anyone reads,
            // or the pipe would never end.
            let mut command = command(&run_args(mode, source, destination));
            command.stdout(writer.try_clone()?).stderr(writer);
            command.spawn()?
        };
        Ok((child, Box::new(LogTee { reader, log })))
    }

    struct LogTee {
        reader: PipeReader,
        log: File,
    }

    impl Read for LogTee {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.reader.read(buf)?;
            self.log.write_all(&buf[..n])?;
            Ok(n)
        }
    }

    /// A process killed by a signal has no exit code: it reads as 128 + the signal, like a shell.
    pub fn exit_code(status: ExitStatus) -> io::Result<i32> {
        status
            .code()
            .or_else(|| status.signal().map(|signal| 128 + signal))
            .ok_or_else(|| io::Error::other("rsync terminated without an exit code"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Real `rsync -rt -8 --itemize-changes --delete --dry-run A/ B/` stdout, captured on the CI
    // runners of 2026-11-04: macOS 26 (openrsync, protocol 29) and Ubuntu (rsync 3.4.1).
    // A and B hold new.txt, `with space.txt`, `è già.txt`, sub/deep.txt (new), updated.txt (older
    // in B), same.txt (identical), a symlink (skipped); B also holds only-in-b.txt,
    // sub/only-in-b-too.txt and old-dir/inside.txt (extra).
    const MACOS: &str = include_str!("../tests/fixtures/rsync_preview_macos.txt");
    const LINUX: &str = include_str!("../tests/fixtures/rsync_preview_linux.txt");
    // The same dry run into a B that does not exist: openrsync prints no `+` flags there.
    const MACOS_NO_B: &str =
        include_str!("../tests/fixtures/rsync_preview_macos_missing_destination.txt");

    fn sizes(name: &str) -> u64 {
        HashMap::from([("new.txt", 7), ("updated.txt", 22), ("sub/deep.txt", 12)])
            .get(name)
            .copied()
            .unwrap_or(1)
    }

    #[test]
    fn both_rsync_flavours_give_the_same_preview() {
        for (platform, log) in [("openrsync", MACOS), ("GNU rsync", LINUX)] {
            let preview = parse(log, true, sizes);

            assert_eq!(
                preview.new.files,
                ["new.txt", "with space.txt", "è già.txt", "sub/deep.txt"],
                "{platform}"
            );
            assert_eq!(preview.updated.files, ["updated.txt"], "{platform}");
            let mut deleted = preview.deleted.files.clone();
            deleted.sort();
            assert_eq!(
                deleted,
                [
                    "old-dir/inside.txt",
                    "only-in-b.txt",
                    "sub/only-in-b-too.txt"
                ],
                "{platform}: folders and notices are not files"
            );
            assert_eq!(preview.counts().new, 4);
            assert_eq!(preview.counts().deleted, 3);
            assert_eq!(preview.bytes, 7 + 1 + 1 + 12 + 22, "{platform}");
            assert_eq!(preview.blocked, None);
        }
    }

    #[test]
    fn a_destination_that_does_not_exist_makes_every_file_new() {
        let preview = parse(MACOS_NO_B, false, sizes);
        assert_eq!(preview.new.count, 6);
        assert_eq!(preview.updated.count, 0);
        assert_eq!(preview.deleted.count, 0);
        // The same lines into a B that exists are not new: no `+` flags.
        assert_eq!(parse(MACOS_NO_B, true, sizes).updated.count, 6);
    }

    #[test]
    fn files_are_classified_by_itemize_code_not_by_text() {
        let log = "\
>f..t...... only-time.txt\n\
>f.st...... size-and-time.txt\n\
>f+++++++++ brand-new.txt\n\
>f+++++++ openrsync-new.txt\n\
>f+++++++++ >f+++ tricky name.txt\n\
cd+++++++++ dir/\n\
cL+++++++++ link -> target\n\
created directory /tmp/B\n\
*deleting   gone.txt\n\
*deleting   gone-dir/\n\
sending incremental file list\n";
        let preview = parse(log, true, |_| 0);
        assert_eq!(
            preview.new.files,
            [
                "brand-new.txt",
                "openrsync-new.txt",
                ">f+++ tricky name.txt"
            ]
        );
        assert_eq!(
            preview.updated.files,
            ["only-time.txt", "size-and-time.txt"]
        );
        assert_eq!(preview.deleted.files, ["gone.txt"]);
    }

    #[test]
    fn crlf_and_an_empty_log_are_fine() {
        assert_eq!(parse("", true, |_| 0), Preview::default());
        let preview = parse(">f+++++++++ a.txt\r\n*deleting   b.txt\r\n", true, |_| 0);
        assert_eq!(preview.new.files, ["a.txt"]);
        assert_eq!(preview.deleted.files, ["b.txt"]);
    }

    #[test]
    fn copy_never_deletes_and_mirror_does_and_a_preview_is_a_dry_run() {
        let copy = run_args(SyncMode::Copy, "/a", "/b");
        let mirror = run_args(SyncMode::Mirror, "/a", "/b");
        for args in [&copy, &mirror] {
            assert!(args.contains(&"--itemize-changes".to_string()));
            assert!(!args.contains(&"--dry-run".to_string()));
            assert!(
                !args.contains(&"-a".to_string()),
                "archive mode needs permissions and owners"
            );
        }
        assert!(!copy.contains(&"--delete".to_string()));
        assert!(mirror.contains(&"--delete".to_string()));

        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let preview = preview_args(mode, "/a", "/b");
            assert!(preview.contains(&"--dry-run".to_string()));
            assert_eq!(
                preview.contains(&"--delete".to_string()),
                mode == SyncMode::Mirror
            );
        }
    }

    #[test]
    fn folders_end_with_a_slash_so_the_content_of_a_goes_into_b() {
        let args = preview_args(SyncMode::Copy, "/Users/anna/Foto", "/Volumes/Backup/");
        assert_eq!(
            args[args.len() - 2..],
            ["/Users/anna/Foto/", "/Volumes/Backup/"]
        );
    }

    #[test]
    fn only_exit_0_is_success() {
        assert!(is_success(0));
        for code in [1, 3, 10, 11, 12, 13, 23, 24, 30, 130] {
            assert!(!is_success(code), "exit {code}");
        }
    }

    #[test]
    fn progress_counts_file_lines_and_names_the_last() {
        for line in [">f+++++++++ sub/è già.txt", "*deleting   sub/è già.txt"] {
            assert!(is_file_line(line));
            assert_eq!(file_name(line), Some("è già.txt"));
        }
        for line in [
            "cd+++++++++ sub/",
            "*deleting   sub/",
            "skipping non-regular file \"l\"",
            "rsync: mkdir failed",
            "",
        ] {
            assert!(!is_file_line(line), "{line}");
        }
        assert_eq!(
            partial_count(LINUX, "", ""),
            4,
            "5 files, the last is in flight"
        );
        assert_eq!(partial_count(MACOS, "", ""), 4);
        assert_eq!(partial_count("", "", ""), 0);
    }

    #[test]
    fn causes_come_from_the_exit_code_and_the_state_of_the_folders() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("A");
        let b = temp.path().join("B");
        std::fs::create_dir(&a).unwrap();
        let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
        let cause = |code, folder| Some(Cause { code, folder });

        // B is not there, A is: 23 or 11 are about B; with A gone it is about A.
        assert_eq!(find_cause(11, "", a, b), cause(3, Folder::B));
        assert_eq!(find_cause(23, "", a, b), cause(3, Folder::B));
        let gone = temp.path().join("gone");
        assert_eq!(
            find_cause(23, "", gone.to_str().unwrap(), b),
            cause(3, Folder::A)
        );
        // The other side went away.
        assert_eq!(find_cause(12, "", a, b), cause(53, Folder::B));
        assert_eq!(
            find_cause(10, "", gone.to_str().unwrap(), a),
            cause(53, Folder::A)
        );
        // Both folders are there: nothing to say, so the generic message is used.
        std::fs::create_dir(b).unwrap();
        for code in [1, 3, 11, 13, 23, 24] {
            assert_eq!(find_cause(code, "", a, b), None, "exit {code}");
        }
    }
}
