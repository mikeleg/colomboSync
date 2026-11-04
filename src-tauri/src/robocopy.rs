//! Robocopy engine: argument builder, exit-code rules, `/UNILOG` decoding and, on Windows, the
//! hidden process calls. The text rules are plain functions, tested on every platform.

use crate::engine::SyncMode;
use crate::error_messages::{self, Cause};
use crate::preview_parser::{self, Preview};

/// Arguments of the `/L` dry run. `log` receives the UTF-16LE `/UNILOG`.
pub fn preview_args(mode: SyncMode, source: &str, destination: &str, log: &str) -> Vec<String> {
    let mut args = base_args(mode, source, destination);
    if mode == SyncMode::Copy {
        // Keep extras in B out of the output: Copy never touches them.
        args.push("/XX".into());
    }
    args.push("/L".into());
    args.extend(output_args(log));
    args
}

/// Arguments of the real run. `log` is the per-run log kept in `logs\`.
///
/// `/TEE` also streams every file line to stdout, in the console OEM code page (the UI follows
/// the copy from there; `/UNICODE` does not change that on a pipe). Copy adds `/XX` so extras
/// in B are not listed:
/// every file line on stdout is then a file of A.
pub fn run_args(mode: SyncMode, source: &str, destination: &str, log: &str) -> Vec<String> {
    let mut args = base_args(mode, source, destination);
    if mode == SyncMode::Copy {
        args.push("/XX".into());
    }
    args.push("/TEE".into());
    args.extend(output_args(log));
    args
}

fn base_args(mode: SyncMode, source: &str, destination: &str) -> Vec<String> {
    let mode_flag = match mode {
        SyncMode::Copy => "/E",
        SyncMode::Mirror => "/MIR",
    };
    vec![
        source.into(),
        destination.into(),
        mode_flag.into(),
        "/R:1".into(),
        "/W:1".into(),
    ]
}

fn output_args(log: &str) -> Vec<String> {
    ["/NJH", "/NJS", "/NDL", "/NC", "/BYTES", "/FP", "/NP"]
        .iter()
        .map(|a| a.to_string())
        .chain([format!("/UNILOG:{log}")])
        .collect()
}

/// Exit codes are a bitmask: 0-7 mean the job worked, 8 and above mean failure.
pub fn is_success(exit_code: i32) -> bool {
    (0..8).contains(&exit_code)
}

/// The preview of a `/L` log; B decides whether a file of A is New or Updated.
pub fn parse_preview(log: &str, source: &str, destination: &str) -> Preview {
    preview_parser::parse(log, source, destination, |p| p.exists())
}

/// Files copied before a failed or stopped run ended. The last file line is the file in flight
/// (possibly cut short), so it is not counted.
pub fn partial_count(log: &str, source: &str, destination: &str) -> usize {
    preview_parser::count_source_files(log, source, destination).saturating_sub(1)
}

/// The mapped Windows error in the log (`ERROR 53 (0x35)`); the exit code says only "failed".
pub fn find_cause(_exit_code: i32, log: &str, source: &str, destination: &str) -> Option<Cause> {
    error_messages::find_cause(log, source, destination)
}

/// `/UNILOG` is UTF-16LE with BOM: never read it as UTF-8.
pub fn decode_unilog(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    String::from_utf16_lossy(&units)
}

/// A run's log as text: the `/UNILOG` of a robocopy run.
pub fn decode_log(bytes: &[u8]) -> String {
    decode_unilog(bytes)
}

#[cfg(windows)]
pub use process::{dry_run, exit_code, spawn};

#[cfg(windows)]
mod process {
    use super::*;
    use crate::engine::Finished;
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
        let text = fs::read(&log)
            .map(|bytes| decode_unilog(&bytes))
            .unwrap_or_default();
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(decode_unilog(&bytes), "è già\r\n");
    }

    #[test]
    fn partial_count_skips_the_file_in_flight() {
        let line = |name: &str| format!("\t  \t\t 5\tD:\\Foto\\{name}\r\n");
        let log = line("a.txt") + &line("b.txt") + &line("c.txt");
        assert_eq!(partial_count(&log, r"D:\Foto", r"E:\Backup"), 2);
        assert_eq!(partial_count(&line("a.txt"), r"D:\Foto", r"E:\Backup"), 0);
        assert_eq!(partial_count("", r"D:\Foto", r"E:\Backup"), 0);
    }
}
