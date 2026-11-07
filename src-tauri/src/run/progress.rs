//! Follows a running copy through its stdout: counts file lines, remembers the file being
//! copied. Which lines are file lines, and what their name is, is the engine's to say.

use crate::engine;
use crate::shared::copy_text;
use serde::Serialize;

/// What the UI shows while a Run is going; sent as the `run-progress` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub sync_name: String,
    pub done: usize,
    pub current_file: String,
    /// Time left, in words ("Mancano circa 28 secondi.").
    pub eta: String,
}

impl Progress {
    pub fn new(
        sync_name: String,
        done: usize,
        total: usize,
        elapsed_ms: u64,
        current_file: String,
    ) -> Self {
        Progress {
            eta: copy_text::eta_text(done, total, elapsed_ms),
            sync_name,
            done,
            current_file,
        }
    }
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
        engine::file_name(&self.last_file_line)
    }

    fn end_line(&mut self) {
        if engine::is_file_line(&self.line) {
            self.done += 1;
            std::mem::swap(&mut self.line, &mut self.last_file_line);
        }
        self.line.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_event_carries_the_time_left_in_words() {
        let progress = Progress::new("Foto".into(), 40, 151, 10_000, "a.jpg".into());
        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json["eta"], "Mancano circa 28 secondi.");

        let json =
            serde_json::to_value(Progress::new("Foto".into(), 0, 151, 0, String::new())).unwrap();
        assert_eq!(json["eta"], "Calcolo il tempo che manca…");
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    // Real output of a run with /TEE on a pipe: a header line, a blank line, then one tab-led
    // line per file.
    const STDOUT: &[u8] = b"\r\n Log File : C:\\L\\r.log\r\n\r\n\t  \t\t       5\tC:\\A\\due.txt\r\n\t  \t\t       5\tC:\\A\\uno.txt\r\n\t  \t\t       5\tC:\\A\\sub\\tre.txt\r\n";

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
