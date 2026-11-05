//! rsync engine (macOS and Linux): argument builder, itemize-line parser, exit-code rules and
//! the process calls. The text rules are plain functions, so they are tested on every platform;
//! only the process calls are Unix-only. Facts and per-platform quirks: `docs/agents/rsync.md`.

use super::preview_parser::Preview;
use crate::shared::domain::{Cause, Folder, SyncMode};
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
fn item(line: &str, padded: bool) -> Option<Item<'_>> {
    let line = line.trim_end_matches('\r');
    let (code, rest) = line.split_once(' ')?;
    if code == "*deleting" {
        // GNU rsync pads the word to the width of an itemize code (11) before the separating
        // space, openrsync does not. The padding is cut by width, not trimmed: a name may start
        // with spaces.
        let name = if padded {
            rest.strip_prefix("  ").unwrap_or(rest)
        } else {
            rest
        };
        return (!name.is_empty() && !name.ends_with('/')).then_some(Item::Deleted { name });
    }
    // The code is `>f` plus attribute letters; its length differs (11 characters on GNU rsync,
    // 9 on openrsync), so only the start is read. One space separates it from the name.
    (code.starts_with(">f") && !rest.is_empty() && !rest.ends_with('/'))
        .then_some(Item::Copied { code, name: rest })
}

/// Which rsync wrote this log: GNU pads `*deleting`, openrsync does not. Told by the width of
/// the first itemize code (11 characters on GNU, 9 on openrsync); a log with no other code than
/// `*deleting` is taken to come from this platform's rsync.
fn is_padded(log: &str) -> bool {
    log.lines()
        .filter_map(|line| line.split(' ').next())
        .find(|code| {
            (code.starts_with(">f") || code.starts_with("cd")) && matches!(code.len(), 9 | 11)
        })
        .map_or(NATIVE_PADDED, |code| code.len() == 11)
}

/// GNU rsync everywhere but on macOS, which ships openrsync.
const NATIVE_PADDED: bool = !cfg!(target_os = "macos");

/// `destination_exists` is `false` when B is not there yet: openrsync's dry run then prints no
/// `+` flags, so every copied file is New. `size_of` gives the size of a file of A, by its path
/// relative to A (itemize lines carry no sizes). `is_folder_of_b` tells which deleted names are
/// folders: openrsync on macOS 15 prints them without the trailing `/`.
pub fn parse(
    log: &str,
    destination_exists: bool,
    size_of: impl Fn(&str) -> u64,
    is_folder_of_b: impl Fn(&str) -> bool,
) -> Preview {
    let mut preview = Preview::default();
    let padded = is_padded(log);
    for line in log.lines() {
        match item(line, padded) {
            Some(Item::Copied { code, name }) => {
                if destination_exists && !code.contains('+') {
                    preview.updated.add(name);
                } else {
                    preview.new.add(name);
                }
                preview.bytes += size_of(name);
            }
            Some(Item::Deleted { name }) if !is_folder_of_b(name) => preview.deleted.add(name),
            Some(Item::Deleted { .. }) | None => {}
        }
    }
    preview
}

/// The preview of a dry run's log; sizes and the existence of B are read from the disk.
pub fn parse_preview(log: &str, source: &str, destination: &str) -> Preview {
    let source = Path::new(source.trim());
    let destination = Path::new(destination.trim());
    parse(
        log,
        destination.is_dir(),
        |name| std::fs::metadata(source.join(name)).map_or(0, |meta| meta.len()),
        |name| std::fs::symlink_metadata(destination.join(name)).is_ok_and(|meta| meta.is_dir()),
    )
}

/// Files copied before a failed or stopped run ended. The log lists a file when rsync starts on
/// it, so when the run was cut short the last file line is the file in flight (possibly cut
/// short itself) and is not counted. Not so when rsync ran to the end (exit 23 and 24: some
/// files were skipped or vanished) or when the last line is a deletion.
pub fn partial_count(
    log: &str,
    _source: &str,
    _destination: &str,
    exit_code: i32,
    cancelled: bool,
) -> usize {
    let (mut copied, mut last_is_copy) = (0, false);
    for line in log.lines() {
        match item(line, NATIVE_PADDED) {
            Some(Item::Copied { .. }) => (copied, last_is_copy) = (copied + 1, true),
            Some(Item::Deleted { .. }) => last_is_copy = false,
            None => {}
        }
    }
    let ran_to_the_end = !cancelled && matches!(exit_code, 23 | 24);
    if last_is_copy && !ran_to_the_end {
        copied - 1
    } else {
        copied
    }
}

/// A run's log is plain UTF-8 (rsync output as the app captured it).
pub fn decode_log(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Is this stdout line a file the run has just started on (or deleted)? Used by the progress.
pub fn is_file_line(line: &str) -> bool {
    item(line, NATIVE_PADDED).is_some()
}

/// File name (no folders) of an itemize line.
pub fn file_name(line: &str) -> Option<&str> {
    let name = match item(line, NATIVE_PADDED)? {
        Item::Copied { name, .. } | Item::Deleted { name } => name,
    };
    name.rsplit('/').next()
}

/// How a folder looks right now. B may well not exist yet (the run creates it), so it is
/// `Missing` only when its parent is missing too: an unplugged disk, not a new folder.
#[derive(PartialEq)]
enum Reach {
    Fine,
    Missing,
    Unreachable,
}

fn reach(folder: Folder, path: &str) -> Reach {
    match std::fs::metadata(path.trim()) {
        Ok(_) => Reach::Fine,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if folder == Folder::B && !parent_is_missing(path) {
                Reach::Fine
            } else {
                Reach::Missing
            }
        }
        // Refused is not gone: the permission check tells that apart.
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Reach::Fine,
        Err(_) => Reach::Unreachable,
    }
}

/// openrsync creates every missing folder above B, which would fill the computer's own disk when
/// B is on an unplugged one (`/Volumes/Backup/Foto`). Only B itself may be new.
pub fn parent_is_missing(destination: &str) -> bool {
    let destination = Path::new(destination.trim());
    !destination.exists()
        && destination
            .parent()
            .is_some_and(|parent| !parent.as_os_str().is_empty() && !parent.exists())
}

/// The reason of a failed rsync, in the Windows-style codes the messages of `error_messages`
/// are keyed on (2/3 not found, 5 no permission, 53 unreachable). It is read from the exit
/// code and from what the two folders are like right now, never from rsync's English text.
/// Exit 23 and 11 cover many things, so they are only explained when a folder is missing or
/// cannot be opened; the socket and stream codes (10, 12, 30, 35) only when a folder really is
/// out of reach. Every other failure stays generic ("apri il registro"). `_log` is there
/// because robocopy's cause is in its log; rsync's text is never read.
pub fn find_cause(exit_code: i32, _log: &str, source: &str, destination: &str) -> Option<Cause> {
    let folders = [(Folder::A, source), (Folder::B, destination)];
    let broken = folders
        .iter()
        .map(|(folder, path)| (*folder, reach(*folder, path)))
        .find(|(_, reach)| *reach != Reach::Fine);
    match (exit_code, broken) {
        (10 | 12 | 30 | 35, Some((folder, _))) => Some(Cause { code: 53, folder }),
        (3 | 11 | 23, Some((folder, Reach::Missing))) => Some(Cause { code: 3, folder }),
        (3 | 11 | 23, _) => folders
            .iter()
            .find(|(_, path)| {
                std::fs::read_dir(path.trim())
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied)
            })
            .map(|(folder, _)| Cause {
                code: 5,
                folder: *folder,
            }),
        _ => None,
    }
}

/// Copies what is read from `reader` into `log`, best effort: once the log cannot be written
/// it is dropped and the reading goes on. Never stopping to read is what keeps rsync from
/// blocking on a full pipe.
pub struct LogTee<R, W> {
    reader: R,
    log: Option<W>,
}

impl<R, W> LogTee<R, W> {
    pub fn new(reader: R, log: Option<W>) -> Self {
        Self { reader, log }
    }
}

impl<R: std::io::Read, W: std::io::Write> std::io::Read for LogTee<R, W> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.reader.read(buf)?;
        if let Some(log) = &mut self.log
            && log.write_all(&buf[..n]).is_err()
        {
            self.log = None;
        }
        Ok(n)
    }
}

#[cfg(unix)]
pub use process::{dry_run, exit_code, spawn};

#[cfg(unix)]
mod process {
    use super::*;
    use crate::engine::Finished;
    use std::fs::File;
    use std::io::{self, Read};
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Child, Command, ExitStatus, Stdio};

    /// rsync exit code for "error in file IO": what GNU rsync says when B cannot be made.
    const FILE_IO_ERROR: i32 = 11;

    fn command(args: &[String]) -> Command {
        let mut command = Command::new("rsync");
        command.args(args).stdin(Stdio::null());
        command
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

    /// Starts the real run. stdout and stderr go to one channel; what is read from it is also
    /// written to `log`, which is the run's log for the history.
    pub fn spawn(
        mode: SyncMode,
        source: &str,
        destination: &str,
        log: &Path,
    ) -> io::Result<(Child, Box<dyn Read + Send>)> {
        let (reader, stdout, stderr) = output_channel()?;
        let child = {
            // The command holds rsync's end of the channel: it is dropped before anyone reads,
            // or the channel would never end.
            let mut command = if parent_is_missing(destination) {
                // Nothing may be created above B: a stand-in that fails like GNU rsync does
                // (exit 11), so the run is recorded as any other failed run.
                let mut stand_in = Command::new("sh");
                stand_in.args(["-c", "exit 11"]).stdin(Stdio::null());
                stand_in
            } else {
                command(&run_args(mode, source, destination))
            };
            command.stdout(stdout).stderr(stderr);
            command.spawn()?
        };
        Ok((child, Box::new(LogTee::new(reader, File::create(log).ok()))))
    }

    /// What the app reads, and rsync's stdout and stderr. A pipe, except on macOS: openrsync
    /// buffers stdout in full when it is not a terminal, so no line would arrive before it
    /// exits and the progress would stand still. A pseudo-terminal makes it write line by line.
    #[cfg(not(target_os = "macos"))]
    fn output_channel() -> io::Result<(Box<dyn Read + Send>, Stdio, Stdio)> {
        let (reader, writer) = io::pipe()?;
        Ok((
            Box::new(reader),
            Stdio::from(writer.try_clone()?),
            Stdio::from(writer),
        ))
    }

    #[cfg(target_os = "macos")]
    fn output_channel() -> io::Result<(Box<dyn Read + Send>, Stdio, Stdio)> {
        let (master, slave) = pty()?;
        Ok((
            Box::new(master),
            Stdio::from(slave.try_clone()?),
            Stdio::from(slave),
        ))
    }

    /// A pseudo-terminal: (master, slave). Neither end leaks into rsync, which gets the slave as
    /// stdout and stderr; a master that rsync inherited would keep the read from ever ending.
    #[cfg(target_os = "macos")]
    fn pty() -> io::Result<(File, std::os::fd::OwnedFd)> {
        use std::ffi::{c_char, c_int, c_void};
        use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

        unsafe extern "C" {
            fn openpty(
                master: *mut c_int,
                slave: *mut c_int,
                name: *mut c_char,
                terminal_settings: *const c_void,
                window_size: *const c_void,
            ) -> c_int;
            fn fcntl(fd: c_int, command: c_int, ...) -> c_int;
        }
        const F_SETFD: c_int = 2;
        const FD_CLOEXEC: c_int = 1;

        let (mut master, mut slave) = (0, 0);
        // SAFETY: `openpty` writes two new file descriptors, which are taken over right away
        // by `OwnedFd`; `fcntl` only sets a flag on descriptors that are open.
        unsafe {
            if openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
            ) != 0
            {
                return Err(io::Error::last_os_error());
            }
            let (master, slave) = (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave));
            fcntl(master.as_raw_fd(), F_SETFD, FD_CLOEXEC);
            fcntl(slave.as_raw_fd(), F_SETFD, FD_CLOEXEC);
            Ok((File::from(master), slave))
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
    const MACOS: &str = include_str!("../../tests/fixtures/rsync_preview_macos.txt");
    const LINUX: &str = include_str!("../../tests/fixtures/rsync_preview_linux.txt");
    // The same dry run into a B that does not exist: openrsync prints no `+` flags there.
    const MACOS_NO_B: &str =
        include_str!("../../tests/fixtures/rsync_preview_macos_missing_destination.txt");

    fn sizes(name: &str) -> u64 {
        HashMap::from([("new.txt", 7), ("updated.txt", 22), ("sub/deep.txt", 12)])
            .get(name)
            .copied()
            .unwrap_or(1)
    }

    #[test]
    fn both_rsync_flavours_give_the_same_preview() {
        for (platform, log) in [("openrsync", MACOS), ("GNU rsync", LINUX)] {
            let preview = parse(log, true, sizes, |_| false);

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
        let preview = parse(MACOS_NO_B, false, sizes, |_| false);
        assert_eq!(preview.new.count, 6);
        assert_eq!(preview.updated.count, 0);
        assert_eq!(preview.deleted.count, 0);
        // The same lines into a B that exists are not new: no `+` flags.
        assert_eq!(parse(MACOS_NO_B, true, sizes, |_| false).updated.count, 6);
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
        let preview = parse(log, true, |_| 0, |_| false);
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
        assert_eq!(parse("", true, |_| 0, |_| false), Preview::default());
        let preview = parse(
            ">f+++++++++ a.txt\r\n*deleting   b.txt\r\n",
            true,
            |_| 0,
            |_| false,
        );
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
            partial_count(LINUX, "", "", 1, true),
            4,
            "5 files, the last is in flight"
        );
        assert_eq!(partial_count(MACOS, "", "", 1, true), 4);
        assert_eq!(partial_count("", "", "", 1, true), 0);
    }

    #[test]
    fn causes_come_from_the_exit_code_and_the_state_of_the_folders() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("A");
        let b = temp.path().join("B");
        std::fs::create_dir(&a).unwrap();
        let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
        let cause = |code, folder| Some(Cause { code, folder });

        // B is not there yet but its parent is: that is a normal first run, not a cause.
        for code in [3, 10, 11, 12, 23] {
            assert_eq!(find_cause(code, "", a, b), None, "exit {code}");
        }
        // B whose parent is missing is an unplugged disk; with A gone it is about A.
        let lost = temp.path().join("Disco").join("B");
        let lost = lost.to_str().unwrap();
        for code in [3, 11, 23] {
            assert_eq!(find_cause(code, "", a, lost), cause(3, Folder::B));
        }
        let gone = temp.path().join("gone");
        let gone = gone.to_str().unwrap();
        assert_eq!(find_cause(23, "", gone, b), cause(3, Folder::A));
        // The other side went away, and a folder really is out of reach.
        assert_eq!(find_cause(12, "", a, lost), cause(53, Folder::B));
        assert_eq!(find_cause(10, "", gone, a), cause(53, Folder::A));
        // Both folders are there: nothing to say, so the generic message is used.
        std::fs::create_dir(b).unwrap();
        for code in [1, 3, 10, 11, 12, 13, 23, 24, 30, 35] {
            assert_eq!(find_cause(code, "", a, b), None, "exit {code}");
        }
    }

    #[test]
    fn a_deleted_name_keeps_its_leading_spaces_on_both_rsync_flavours() {
        // GNU pads `*deleting` to 11 characters and a space; openrsync adds one space.
        let gnu = ">f+++++++++ new.txt\n*deleting     vecchio.txt\n*deleting   altro.txt\n";
        let open = ">f+++++++ new.txt\n*deleting   vecchio.txt\n*deleting altro.txt\n";
        for (log, name) in [(gnu, "GNU"), (open, "openrsync")] {
            let preview = parse(log, true, |_| 0, |_| false);
            assert_eq!(
                preview.deleted.files,
                ["  vecchio.txt", "altro.txt"],
                "{name}"
            );
        }
    }

    #[test]
    fn a_deleted_folder_without_a_trailing_slash_is_not_a_file() {
        // openrsync on macOS 15 prints `*deleting old-dir` (macOS 26 and GNU print `old-dir/`),
        // so B tells which deleted names are folders.
        let log = ">f+++++++ new.txt\n*deleting only-in-b.txt\n*deleting old-dir/inside.txt\n*deleting old-dir\n";
        let preview = parse(log, true, |_| 0, |name| name == "old-dir");
        assert_eq!(
            preview.deleted.files,
            ["only-in-b.txt", "old-dir/inside.txt"]
        );
        assert_eq!(preview.deleted.count, 2);
    }

    #[test]
    fn the_file_in_flight_is_not_counted_only_when_the_run_was_cut_short() {
        let log = ">f+++++++++ a.txt\n>f+++++++++ b.txt\n>f+++++++++ c.txt\n";
        // Killed or aborted mid-transfer: the last one is not done.
        assert_eq!(partial_count(log, "", "", 137, true), 2);
        assert_eq!(partial_count(log, "", "", 12, false), 2);
        // rsync ran to the end with some files skipped or vanished: all were done.
        assert_eq!(partial_count(log, "", "", 23, false), 3);
        assert_eq!(partial_count(log, "", "", 24, false), 3);
        // A stop wins over the exit code.
        assert_eq!(partial_count(log, "", "", 23, true), 2);
        // A deletion is never a file in flight.
        let log = ">f+++++++++ a.txt\n>f+++++++++ b.txt\n*deleting   old.txt\n";
        assert_eq!(partial_count(log, "", "", 137, true), 2);
        assert_eq!(partial_count(log, "", "", 12, false), 2);
    }

    #[test]
    fn a_log_that_cannot_be_written_never_stops_the_reading() {
        use std::io::{Cursor, Read, Write};
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disco pieno"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let output: Vec<u8> = (0..20_000u32).map(|n| (n % 251) as u8).collect();
        let mut tee = LogTee::new(Cursor::new(output.clone()), Some(Broken));
        let mut all = Vec::new();
        tee.read_to_end(&mut all).unwrap();
        assert_eq!(all, output, "rsync's output is drained to the end");

        let mut kept = Vec::new();
        LogTee::new(Cursor::new(output.clone()), Some(&mut kept))
            .read_to_end(&mut Vec::new())
            .unwrap();
        assert_eq!(kept, output, "a working log gets everything");
        let mut no_log = LogTee::new(Cursor::new(output.clone()), None::<Vec<u8>>);
        let mut all = Vec::new();
        no_log.read_to_end(&mut all).unwrap();
        assert_eq!(all, output);
    }
}
