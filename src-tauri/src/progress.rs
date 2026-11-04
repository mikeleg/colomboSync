//! Follows a running copy through its stdout: counts file lines, remembers the file being
//! copied. Labels are never read. robocopy (`/TEE` with `/NJH /NJS /NDL /NC /NP`): a file line is
//! the only kind of line that starts with a tab (the `/TEE` header, blank lines and `ERROR`
//! lines do not). rsync: a file line starts with its itemize code (`>f…`, `*deleting`).

use serde::Serialize;

/// What the UI shows while a Run is going; sent as the `run-progress` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub sync_name: String,
    pub done: usize,
    pub total: usize,
    pub elapsed_ms: u64,
    pub current_file: String,
}

#[derive(Debug, Default)]
pub struct StdoutFollower {
    done: usize,
    line: Vec<u8>,
    /// Raw bytes of the last file line: decoded only when someone asks for the name.
    last_file_line: Vec<u8>,
}

impl StdoutFollower {
    /// Raw bytes as read from the pipe; a line may be split across calls.
    pub fn feed(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte == b'\n' {
                self.end_line();
            } else {
                self.line.push(byte);
            }
        }
    }

    pub fn done(&self) -> usize {
        self.done
    }

    /// File name (no folders) of the last file line.
    pub fn current_file(&self) -> String {
        file_name(&self.last_file_line)
    }

    fn end_line(&mut self) {
        if is_file_line(&self.line) {
            self.done += 1;
            std::mem::swap(&mut self.line, &mut self.last_file_line);
        }
        self.line.clear();
    }
}

#[cfg(windows)]
fn is_file_line(line: &[u8]) -> bool {
    line.first() == Some(&b'\t')
}

#[cfg(windows)]
fn file_name(raw: &[u8]) -> String {
    let text = decode_oem(raw);
    let path = text.rsplit('\t').next().unwrap_or_default().trim_end();
    path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
}

#[cfg(unix)]
fn is_file_line(line: &[u8]) -> bool {
    crate::rsync::is_file_line(&String::from_utf8_lossy(line))
}

#[cfg(unix)]
fn file_name(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw);
    crate::rsync::file_name(&text)
        .unwrap_or_default()
        .to_string()
}

/// Text robocopy writes to a pipe is in the console OEM code page (even with `/UNICODE`).
#[cfg(windows)]
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
        let needed = MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0);
        if needed <= 0 {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let mut wide = vec![0u16; needed as usize];
        MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), len, wide.as_mut_ptr(), needed);
        String::from_utf16_lossy(&wide)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    // Real output of a run with /TEE on a pipe: a header line, a blank line, then one tab-led
    // line per file.
    const STDOUT: &[u8] = b"\r\n Log File : C:\\L\\r.log\r\n\r\n\t  \t\t       5\tC:\\A\\due.txt\r\n\t  \t\t       5\tC:\\A\\uno.txt\r\n\t  \t\t       5\tC:\\A\\sub\\tre.txt\r\n";

    #[test]
    fn oem_bytes_become_text() {
        assert_eq!(decode_oem(b"due.txt"), "due.txt");
        assert_eq!(decode_oem(b""), "");
        // 0x8A is a letter in the usual OEM code pages (437, 850), never a replacement char.
        assert!(!decode_oem(&[0x8A]).contains('\u{FFFD}'));
    }

    #[test]
    fn counts_only_file_lines_and_names_the_last_one() {
        let mut follower = StdoutFollower::default();
        follower.feed(STDOUT);
        assert_eq!(follower.done(), 3);
        assert_eq!(follower.current_file(), "tre.txt");

        let mut follower = StdoutFollower::default();
        follower.feed(&STDOUT[..STDOUT.len() - 10]);
        assert_eq!(follower.done(), 2);
        assert_eq!(follower.current_file(), "uno.txt");
    }

    #[test]
    fn a_line_split_across_reads_counts_once() {
        let mut follower = StdoutFollower::default();
        for chunk in STDOUT.chunks(7) {
            follower.feed(chunk);
        }
        assert_eq!(follower.done(), 3);
        assert_eq!(follower.current_file(), "tre.txt");
    }

    #[test]
    fn errors_and_retry_messages_are_not_files() {
        let mut follower = StdoutFollower::default();
        follower.feed(
            b"2026/11/03 12:00:00 ERROR 5 (0x00000005) Copying File C:\\A\\x.txt\r\nAccess is denied.\r\n\r\n",
        );
        assert_eq!(follower.done(), 0);
        assert_eq!(follower.current_file(), "");
    }

    #[test]
    fn crlf_line_endings_are_handled() {
        let mut follower = StdoutFollower::default();
        follower.feed(b"\t\t 5\tD:\\Foto\\a.txt\r\n");
        assert_eq!(follower.done(), 1);
        assert_eq!(follower.current_file(), "a.txt");
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::*;

    const BEFORE_DELETIONS: &str = "created directory /tmp/B\ncd+++++++++ ./\n>f+++++++++ due.txt\n>f+++++++++ uno.txt\nskipping non-regular file \"l\"\n>f+++++++++ sub/è già.txt\n";

    // What an rsync run prints on its pipe: itemize lines, plus notices that are not files. The
    // padding of `*deleting` is the one of the rsync of this platform.
    fn stdout() -> Vec<u8> {
        let word = if cfg!(target_os = "macos") {
            "*deleting "
        } else {
            "*deleting   "
        };
        format!("{BEFORE_DELETIONS}{word}extra.txt\n{word}old-dir/\n").into_bytes()
    }

    #[test]
    fn counts_only_file_lines_and_names_the_last_one() {
        let stdout = stdout();
        let mut follower = StdoutFollower::default();
        follower.feed(&stdout);
        assert_eq!(
            follower.done(),
            4,
            "three copied, one deleted; folders and notices are not files"
        );
        assert_eq!(follower.current_file(), "extra.txt");

        // Cut inside the first deletion: only what is complete counts.
        let mut follower = StdoutFollower::default();
        follower.feed(&stdout[..BEFORE_DELETIONS.len() + 3]);
        assert_eq!(follower.done(), 3);
        assert_eq!(follower.current_file(), "è già.txt");
    }

    #[test]
    fn a_line_split_across_reads_counts_once() {
        let mut follower = StdoutFollower::default();
        for chunk in stdout().chunks(5) {
            follower.feed(chunk);
        }
        assert_eq!(follower.done(), 4);
        assert_eq!(follower.current_file(), "extra.txt");
    }

    #[test]
    fn errors_are_not_files() {
        let mut follower = StdoutFollower::default();
        follower.feed(b"rsync: [sender] change_dir \"/x\" failed: No such file or directory (2)\nrsync error: some files/attrs were not transferred (see previous errors) (code 23)\n");
        assert_eq!(follower.done(), 0);
        assert_eq!(follower.current_file(), "");
    }
}
