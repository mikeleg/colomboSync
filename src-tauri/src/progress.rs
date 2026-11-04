//! Follows a running robocopy through its stdout (`/TEE`): counts file lines, remembers the
//! file being copied. Labels are never read: with `/NJH /NJS /NDL /NC /NP` a file line is the
//! only kind of line that starts with a tab (the `/TEE` header, blank lines and `ERROR` lines
//! do not).

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
        let text = decode_oem(&self.last_file_line);
        let path = text.rsplit('\t').next().unwrap_or_default().trim_end();
        path.rsplit(['\\', '/']).next().unwrap_or(path).to_string()
    }

    fn end_line(&mut self) {
        if self.line.first() == Some(&b'\t') {
            self.done += 1;
            std::mem::swap(&mut self.line, &mut self.last_file_line);
        }
        self.line.clear();
    }
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
        let needed = MultiByteToWideChar(CP_OEMCP, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0);
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
