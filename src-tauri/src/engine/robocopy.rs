//! Robocopy engine: argument builder, exit-code rules, `/UNILOG` decoding and, on Windows, the
//! hidden process calls. The text rules are plain functions, tested on every platform.

use super::preview_parser::{self, Preview};
use crate::shared::domain::{Cause, Folder, SyncMode};
use crate::shared::error_messages::MAPPED_CODES;
use crate::shared::path_rules;
use std::path::Path;

/// Arguments of the `/L` dry run. `log` receives the UTF-16LE `/UNILOG`.
pub fn preview_args(mode: SyncMode, source: &str, destination: &str, log: &str) -> Vec<String> {
    args(mode, source, destination, log, true)
}

/// Arguments of the real run. `log` is the per-run log kept in `logs\`.
pub fn run_args(mode: SyncMode, source: &str, destination: &str, log: &str) -> Vec<String> {
    args(mode, source, destination, log, false)
}

/// Shared builder for both runs. `dry_run` adds `/L`, otherwise `/TEE`. `/TEE` streams every file
/// line to stdout in the console OEM code page; the UI follows the copy from there, and `/UNICODE`
/// does not change that on a pipe. Copy adds `/XX` so extras in B are not listed: every file line
/// on stdout is then a file of A.
fn args(mode: SyncMode, source: &str, destination: &str, log: &str, dry_run: bool) -> Vec<String> {
    let mode_flag = match mode {
        SyncMode::Copy => "/E",
        SyncMode::Mirror => "/MIR",
    };
    let mut args: Vec<String> = [source, destination, mode_flag, "/R:1", "/W:1"]
        .map(String::from)
        .into();
    if mode == SyncMode::Copy {
        // Keep extras in B out of the output: Copy never touches them.
        args.push("/XX".into());
    }
    args.push(if dry_run { "/L" } else { "/TEE" }.into());
    args.extend(["/NJH", "/NJS", "/NDL", "/NC", "/BYTES", "/FP", "/NP"].map(String::from));
    args.push(format!("/UNILOG:{log}"));
    args
}

/// Exit codes are a bitmask: 0-7 mean the job worked, 8 and above mean failure.
pub fn is_success(exit_code: i32) -> bool {
    (0..8).contains(&exit_code)
}

/// The preview of a `/L` log; B decides whether a file of A is New or Updated. When B is not there
/// yet nothing is in it, so every file is New without looking at the disk once per file.
pub fn parse_preview(log: &str, source: &str, destination: &str) -> Preview {
    let destination_exists = Path::new(destination.trim()).is_dir();
    preview_parser::parse(log, source, destination, |p| {
        destination_exists && p.exists()
    })
}

/// Files copied before a failed or stopped run ended. The last file line is the file in flight
/// (possibly cut short), so it is not counted.
pub fn partial_count(
    log: &str,
    source: &str,
    destination: &str,
    _exit_code: i32,
    _cancelled: bool,
) -> usize {
    preview_parser::count_source_files(log, source, destination).saturating_sub(1)
}

/// The mapped Windows error in the log (`ERROR 53 (0x35)`); the exit code says only "failed".
pub fn find_cause(_exit_code: i32, log: &str, source: &str, destination: &str) -> Option<Cause> {
    let (code, line) = first_mapped_error(log)?;
    Some(Cause {
        code,
        folder: folder_of(line, source, destination),
    })
}

/// First log line holding `<n> (0x<hex>)` with a mapped `n`, and that `n`. Any label before
/// the number (`ERROR`, `ERRORE`…) is ignored.
fn first_mapped_error(log: &str) -> Option<(u32, &str)> {
    log.lines().find_map(|line| {
        let code = error_codes(line).find(|code| MAPPED_CODES.contains(code))?;
        Some((code, line))
    })
}

/// Is this log line an error of robocopy? It holds a Windows error code (`ERROR 5 (0x00000005)`);
/// the label before it is localized and never read. A file line (it starts with a tab) is a file
/// whatever its name says.
pub fn is_error_line(line: &str) -> bool {
    !line.starts_with('\t') && error_codes(line).next().is_some()
}

/// The `n` of every `<n> (0x<hex>)` in a log line.
fn error_codes(line: &str) -> impl Iterator<Item = u32> + '_ {
    let tokens = line.split_whitespace();
    tokens
        .clone()
        .zip(tokens.skip(1))
        .filter_map(|(number, hex)| {
            let hex = hex.strip_prefix("(0x")?.strip_suffix(')')?;
            let is_hex = !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
            is_hex.then(|| number.parse().ok()).flatten()
        })
}

/// The folder an error line talks about: the one whose path the line holds. A line that
/// names neither (the path is cut, or relative) falls back to the folder most likely to be on
/// the network: A if it is a share and B is not, else B.
fn folder_of(line: &str, source: &str, destination: &str) -> Folder {
    let line = line.replace('/', "\\").to_lowercase();
    match (mentions(&line, source), mentions(&line, destination)) {
        (true, false) => Folder::A,
        (false, true) => Folder::B,
        _ if path_rules::is_unc(source.trim()) && !path_rules::is_unc(destination.trim()) => {
            Folder::A
        }
        _ => Folder::B,
    }
}

/// `line` (lower case, `\` separators) holds `path` as a folder or something inside it: what
/// follows is the end of the line or a `\`, never other characters (`D:\Foto 2` is not in
/// `D:\Foto`), and what comes before is a space, as after a label.
fn mentions(line: &str, path: &str) -> bool {
    let path = path_rules::normalize_windows(path);
    let line = line.trim_end();
    !path.is_empty()
        && line.match_indices(&path).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = line[at + path.len()..].chars().next();
            before.is_none_or(|c| c.is_whitespace() || c == '"') && after.is_none_or(|c| c == '\\')
        })
}

/// A run's log as text: the `/UNILOG` of a robocopy run, UTF-16LE with BOM (never UTF-8).
pub fn decode_log(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    String::from_utf16_lossy(&units)
}

/// How many of the first bytes of `bytes` decode to whole characters: the rest is an odd byte or a
/// high surrogate whose low half is in the next bytes of the log.
pub fn whole_chars_len(bytes: &[u8]) -> usize {
    let even = bytes.len() & !1;
    match even.checked_sub(2) {
        Some(last)
            if (0xD800..0xDC00).contains(&u16::from_le_bytes([bytes[last], bytes[last + 1]])) =>
        {
            last
        }
        _ => even,
    }
}

#[cfg(windows)]
pub use process::{dry_run, exit_code, file_name, is_file_line, spawn};

#[cfg(windows)]
mod process {
    use super::*;
    use crate::engine::{self, Finished};
    use std::fs;
    use std::io::{self, Read};
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, ExitStatus, Stdio};
    use std::sync::atomic::{AtomicU32, Ordering};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn robocopy_exe() -> PathBuf {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        PathBuf::from(root).join("System32").join("robocopy.exe")
    }

    fn command(args: &[String]) -> Command {
        let mut command = Command::new(robocopy_exe());
        command
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stderr(Stdio::null());
        command
    }

    /// Runs the `/L` dry run hidden and blocks until it exits; `log` is its decoded `/UNILOG`.
    pub fn dry_run(mode: SyncMode, source: &str, destination: &str) -> io::Result<Finished> {
        static SEQ: AtomicU32 = AtomicU32::new(0);
        let log = std::env::temp_dir().join(format!(
            "colombosync-anteprima-{}-{}.log",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let args = preview_args(mode, source, destination, &log.to_string_lossy());
        let status = command(&args).stdout(Stdio::null()).status()?;
        let text = engine::read_log(&log);
        let _ = fs::remove_file(&log);
        Ok(Finished {
            code: exit_code(status)?,
            log: text,
        })
    }

    /// Starts the real run hidden with stdout piped; the caller reads the pipe, so it never
    /// blocks. robocopy writes the run's log to `log` itself (`/UNILOG`).
    pub fn spawn(
        mode: SyncMode,
        source: &str,
        destination: &str,
        log: &Path,
    ) -> io::Result<(Child, Box<dyn Read + Send>)> {
        let args = run_args(mode, source, destination, &log.to_string_lossy());
        let mut child = command(&args).stdout(Stdio::piped()).spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("robocopy senza stdout"))?;
        Ok((child, Box::new(stdout)))
    }

    pub fn exit_code(status: ExitStatus) -> io::Result<i32> {
        status
            .code()
            .ok_or_else(|| io::Error::other("robocopy terminated without an exit code"))
    }

    /// Is this stdout line of a running copy (`/TEE` with `/NJH /NJS /NDL /NC /NP`) a file? A file
    /// line is the only kind of line that starts with a tab: the `/TEE` header, blank lines and
    /// `ERROR` lines do not. Labels are never read.
    pub fn is_file_line(line: &[u8]) -> bool {
        line.first() == Some(&b'\t')
    }

    /// File name (no folders) of a file line.
    pub fn file_name(raw: &[u8]) -> String {
        let text = decode_oem(raw);
        let path = text.rsplit('\t').next().unwrap_or_default().trim_end();
        path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
    }

    /// Text robocopy writes to a pipe is in the console OEM code page (even with `/UNICODE`).
    fn decode_oem(bytes: &[u8]) -> String {
        const CP_OEMCP: u32 = 1;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn MultiByteToWideChar(
                code_page: u32,
                flags: u32,
                src: *const u8,
                src_len: i32,
                dst: *mut u16,
                dst_len: i32,
            ) -> i32;
        }
        let Ok(len) = i32::try_from(bytes.len()) else {
            return String::from_utf8_lossy(bytes).into_owned();
        };
        // SAFETY: both calls read `len` bytes from `bytes`; the second writes at most `needed`
        // units into a buffer of that size.
        unsafe {
            let needed =
                MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0);
            if needed <= 0 {
                return String::from_utf8_lossy(bytes).into_owned();
            }
            let mut wide = vec![0u16; needed as usize];
            MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), len, wide.as_mut_ptr(), needed);
            String::from_utf16_lossy(&wide)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn oem_bytes_become_text() {
            assert_eq!(decode_oem(b"due.txt"), "due.txt");
            assert_eq!(decode_oem(b""), "");
            // 0x8A is a letter in the usual OEM code pages (437, 850), never a replacement char.
            assert!(!decode_oem(&[0x8A]).contains('\u{FFFD}'));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_error_line_is_the_one_with_a_windows_error_code_in_any_language() {
        for label in ["ERROR", "ERRORE"] {
            // 32 is not one of the codes that get a message of their own: it is an error all the same.
            for code in [5, 32] {
                let line = format!(
                    "2026/11/04 11:58:26 {label} {code} (0x{code:08X}) Copying File E:\\Backup\\a.jpg\r"
                );
                assert!(is_error_line(&line), "{line}");
            }
        }
        for line in [
            "",
            "Access is denied.",
            "\t  \t\t 5\tD:\\Foto\\a.jpg\r",
            // A file whose name looks like an error code is still a file line.
            "\t\t 5\tD:\\Foto\\ERROR 5 (0x5)",
            "2026/11/04 11:58:26 Waiting 1 seconds... Retrying...",
        ] {
            assert!(!is_error_line(line), "{line:?}");
        }
    }

    #[test]
    fn exit_codes_0_to_7_are_success() {
        for code in 0..=7 {
            assert!(is_success(code), "exit {code} must be success");
        }
        for code in [8, 9, 16] {
            assert!(!is_success(code), "exit {code} must be failure");
        }
    }

    #[test]
    fn full_argument_lists_are_pinned_for_every_mode_and_run() {
        let (src, dst, log) = (r"C:\A", r"D:\B", r"C:\log.txt");
        assert_eq!(
            preview_args(SyncMode::Copy, src, dst, log),
            [
                r"C:\A",
                r"D:\B",
                "/E",
                "/R:1",
                "/W:1",
                "/XX",
                "/L",
                "/NJH",
                "/NJS",
                "/NDL",
                "/NC",
                "/BYTES",
                "/FP",
                "/NP",
                r"/UNILOG:C:\log.txt",
            ]
            .map(String::from)
        );
        assert_eq!(
            run_args(SyncMode::Copy, src, dst, log),
            [
                r"C:\A",
                r"D:\B",
                "/E",
                "/R:1",
                "/W:1",
                "/XX",
                "/TEE",
                "/NJH",
                "/NJS",
                "/NDL",
                "/NC",
                "/BYTES",
                "/FP",
                "/NP",
                r"/UNILOG:C:\log.txt",
            ]
            .map(String::from)
        );
        assert_eq!(
            preview_args(SyncMode::Mirror, src, dst, log),
            [
                r"C:\A",
                r"D:\B",
                "/MIR",
                "/R:1",
                "/W:1",
                "/L",
                "/NJH",
                "/NJS",
                "/NDL",
                "/NC",
                "/BYTES",
                "/FP",
                "/NP",
                r"/UNILOG:C:\log.txt",
            ]
            .map(String::from)
        );
        assert_eq!(
            run_args(SyncMode::Mirror, src, dst, log),
            [
                r"C:\A",
                r"D:\B",
                "/MIR",
                "/R:1",
                "/W:1",
                "/TEE",
                "/NJH",
                "/NJS",
                "/NDL",
                "/NC",
                "/BYTES",
                "/FP",
                "/NP",
                r"/UNILOG:C:\log.txt",
            ]
            .map(String::from)
        );
    }

    #[test]
    fn copy_mode_never_uses_mir() {
        let preview = preview_args(SyncMode::Copy, r"D:\Foto", r"E:\Backup", r"C:\t\p.log");
        let run = run_args(SyncMode::Copy, r"D:\Foto", r"E:\Backup", r"C:\t\r.log");
        for args in [&preview, &run] {
            assert!(args.contains(&"/E".to_string()));
            assert!(!args.contains(&"/MIR".to_string()));
            assert!(!args.contains(&"/PURGE".to_string()));
            assert!(args.contains(&"/R:1".to_string()));
            assert!(args.contains(&"/W:1".to_string()));
        }
    }

    #[test]
    fn mirror_mode_uses_mir() {
        let preview = preview_args(SyncMode::Mirror, r"D:\Foto", r"E:\Backup", r"C:\t\p.log");
        let run = run_args(SyncMode::Mirror, r"D:\Foto", r"E:\Backup", r"C:\t\r.log");
        for args in [&preview, &run] {
            assert!(args.contains(&"/MIR".to_string()));
            assert!(!args.contains(&"/E".to_string()));
            assert!(!args.contains(&"/XX".to_string()), "extras must show up");
            assert!(args.contains(&"/R:1".to_string()));
            assert!(args.contains(&"/W:1".to_string()));
        }
        assert!(preview.contains(&"/L".to_string()));
        assert!(!run.contains(&"/L".to_string()));
        assert!(run.contains(&"/TEE".to_string()));
    }

    #[test]
    fn preview_is_a_dry_run_that_hides_extras_and_run_is_not() {
        let preview = preview_args(SyncMode::Copy, "A", "B", "p.log");
        assert!(preview.contains(&"/L".to_string()));
        assert!(preview.contains(&"/XX".to_string()));
        assert_eq!(preview[..2], ["A", "B"]);
        assert_eq!(preview.last().unwrap(), "/UNILOG:p.log");

        let run = run_args(SyncMode::Copy, "A", "B", "r.log");
        assert!(!run.contains(&"/L".to_string()));
        assert_eq!(run.last().unwrap(), "/UNILOG:r.log");
    }

    #[test]
    fn run_streams_stdout_with_tee_and_never_lists_extras_in_copy() {
        let run = run_args(SyncMode::Copy, "A", "B", "r.log");
        assert!(run.contains(&"/TEE".to_string()));
        assert!(run.contains(&"/XX".to_string()));
        let mirror = run_args(SyncMode::Mirror, "A", "B", "r.log");
        assert!(mirror.contains(&"/TEE".to_string()));
        assert!(!mirror.contains(&"/XX".to_string()));
        let preview = preview_args(SyncMode::Copy, "A", "B", "p.log");
        assert!(!preview.contains(&"/TEE".to_string()));
    }

    #[test]
    fn unilog_is_decoded_as_utf16le_with_bom() {
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "è già\r\n".encode_utf16() {
            bytes.extend(unit.to_le_bytes());
        }
        assert_eq!(decode_log(&bytes), "è già\r\n");
    }

    #[test]
    fn a_log_chunk_ends_whole_before_an_odd_byte_or_a_high_surrogate() {
        let units =
            |text: &str| -> Vec<u8> { text.encode_utf16().flat_map(u16::to_le_bytes).collect() };
        let folder = units("a📁"); // a, high surrogate, low surrogate: 6 bytes

        assert_eq!(whole_chars_len(&folder), 6);
        assert_eq!(whole_chars_len(&folder[..5]), 2);
        assert_eq!(whole_chars_len(&folder[..4]), 2);
        assert_eq!(whole_chars_len(&folder[..3]), 2);
        assert_eq!(whole_chars_len(&folder[..2]), 2);
        assert_eq!(whole_chars_len(&folder[..1]), 0);
        assert_eq!(whole_chars_len(&[]), 0);
    }

    #[test]
    #[cfg(windows)]
    fn a_file_is_updated_only_where_the_destination_exists_and_holds_it() {
        let temp = tempfile::tempdir().unwrap();
        let (a, b) = (temp.path().join("A"), temp.path().join("B"));
        let (a, b) = (a.to_str().unwrap(), b.to_str().unwrap());
        let log = format!("\t\t 5\t{a}\\new.txt\r\n\t\t 5\t{a}\\updated.txt\r\n");

        let without_b = parse_preview(&log, a, b);
        assert_eq!(without_b.new.files, ["new.txt", "updated.txt"]);
        assert_eq!(without_b.updated.count, 0);

        std::fs::create_dir(b).unwrap();
        std::fs::write(format!("{b}\\updated.txt"), "x").unwrap();
        let with_b = parse_preview(&log, a, b);
        assert_eq!(with_b.new.files, ["new.txt"]);
        assert_eq!(with_b.updated.files, ["updated.txt"]);
    }

    #[test]
    fn partial_count_skips_the_file_in_flight() {
        let line = |name: &str| format!("\t  \t\t 5\tD:\\Foto\\{name}\r\n");
        let log = line("a.txt") + &line("b.txt") + &line("c.txt");
        let (a, b) = (r"D:\Foto", r"E:\Backup");
        assert_eq!(partial_count(&log, a, b, 16, false), 2);
        assert_eq!(partial_count(&line("a.txt"), a, b, 16, true), 0);
        assert_eq!(partial_count("", a, b, 16, false), 0);
    }

    const A: &str = r"D:\Foto";
    const NAS_B: &str = r"\\NAS\Backup";

    /// A real log: label wording changes with the Windows language, the code does not.
    fn log(code: u32, label: &str, what: &str, path: &str) -> String {
        format!(
            "\r\n2026/11/04 11:58:26 {label} {code} (0x{code:08X}) {what} {path}\r\nIl messaggio di Windows, in qualunque lingua.\r\n"
        )
    }

    fn english(code: u32, path: &str) -> String {
        log(code, "ERROR", "Accessing Destination Directory", path)
    }

    fn italian(code: u32, path: &str) -> String {
        log(
            code,
            "ERRORE",
            "Accesso alla directory di destinazione",
            path,
        )
    }

    fn at(code: u32, folder: Folder) -> Option<Cause> {
        Some(Cause { code, folder })
    }

    #[test]
    fn error_53_is_found_by_code_in_any_language() {
        for text in [english(53, r"\\NAS\Backup\"), italian(53, r"\\NAS\Backup\")] {
            assert_eq!(find_cause(8, &text, A, NAS_B), at(53, Folder::B));
        }
    }

    #[test]
    fn error_67_is_an_unreachable_share_too() {
        assert_eq!(
            find_cause(8, &italian(67, r"\\NAS\Backup\x\"), A, NAS_B),
            at(67, Folder::B)
        );
    }

    #[test]
    fn error_2_and_3_name_the_folder_that_cannot_be_found() {
        for code in [2, 3] {
            for text in [
                log(code, "ERROR", "Accessing Source Directory", r"D:\Foto\"),
                log(
                    code,
                    "ERRORE",
                    "Accesso alla directory di origine",
                    r"D:\Foto\",
                ),
            ] {
                assert_eq!(find_cause(8, &text, A, r"E:\Backup"), at(code, Folder::A));
            }
        }
    }

    #[test]
    fn error_5_says_access_was_denied_on_that_folder() {
        for text in [
            log(5, "ERROR", "Copying File", r"E:\Backup\2024\a.jpg"),
            log(5, "ERRORE", "Copia file", r"E:\Backup\2024\a.jpg"),
        ] {
            assert_eq!(find_cause(8, &text, A, r"E:\Backup"), at(5, Folder::B));
        }
    }

    #[test]
    fn any_other_code_or_no_code_has_no_cause() {
        for text in [
            english(32, r"E:\Backup\a.jpg"), // sharing violation: not mapped
            "robocopy ha detto qualcosa senza codice\r\n".to_string(),
            String::new(),
        ] {
            assert_eq!(find_cause(8, &text, A, r"E:\Backup"), None);
        }
    }

    #[test]
    fn the_folder_is_found_by_whole_path_not_by_a_name_that_starts_the_same() {
        // `D:\Foto 2` is a folder of its own: the error is about A, not about B (`D:\Foto`).
        let (a, b) = (r"D:\Foto 2", r"D:\Foto");
        let line = english(5, r"D:\Foto 2\a.jpg");
        assert_eq!(find_cause(8, &line, a, b), at(5, Folder::A));
        assert_eq!(
            find_cause(8, &english(5, r"D:\Foto 2\"), a, b),
            at(5, Folder::A)
        );
        assert_eq!(
            find_cause(8, &english(5, r"d:/foto/a.jpg"), a, b),
            at(5, Folder::B)
        );
        // B is `D:\Foto2`: a sibling by name, not a parent.
        let line = english(5, r"D:\Foto2\a.jpg");
        assert_eq!(find_cause(8, &line, A, r"D:\Foto2"), at(5, Folder::B));
    }

    #[test]
    fn a_line_without_a_known_path_blames_the_share_then_b() {
        let line = english(53, r"\\ALTRO\x\");
        assert_eq!(find_cause(8, &line, A, NAS_B), at(53, Folder::B));
        assert_eq!(find_cause(8, &line, NAS_B, r"E:\Backup"), at(53, Folder::A));
        assert_eq!(
            find_cause(8, &english(3, "x"), A, r"E:\Backup"),
            at(3, Folder::B)
        );
    }

    #[test]
    fn the_first_mapped_error_wins() {
        let text = english(32, r"E:\a") + &english(5, r"E:\Backup\b") + &english(53, NAS_B);
        assert_eq!(find_cause(8, &text, A, r"E:\Backup"), at(5, Folder::B));
    }
}
