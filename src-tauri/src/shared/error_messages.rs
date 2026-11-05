//! Why a copy failed, in plain Italian. The cause is read from the Windows error code in the log
//! (`ERROR 53 (0x00000035)`), never from the labels around it: those are localized. Only the
//! cause (code + folder) is kept; the texts are made from it when they are needed.

use super::domain::{Cause, Folder, Outcome, Run};
use super::path_rules;
use serde::Serialize;
use std::io;

/// What the user reads: the sentence that says what happened, and what to do about it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Explanation {
    pub title: String,
    pub advice: String,
}

impl From<&str> for Explanation {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}

impl From<String> for Explanation {
    fn from(message: String) -> Self {
        Explanation {
            title: message,
            advice: String::new(),
        }
    }
}

/// When it failed: the wording differs between a preview (nothing is copied yet) and a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moment {
    Preview,
    Run,
}

pub const STOPPED_NOTE: &str = "La copia era stata fermata a mano: puoi riprenderla quando vuoi.";
pub const GENERIC_NOTE: &str =
    "L’ultima copia non è stata completata. Nel registro trovi i dettagli.";

const MAPPED_CODES: [u32; 5] = [2, 3, 5, 53, 67];
/// `ERROR_PATH_NOT_FOUND`: what a missing folder is, when Windows says only "not found".
const PATH_NOT_FOUND: u32 = 3;
/// `ERROR_ACCESS_DENIED`, which is also what a refused folder is on macOS and Linux.
const ACCESS_DENIED: u32 = 5;
/// The copy engine is not installed (rsync): "command not found" is 127 in a shell. Not a folder
/// problem, so it is only ever made by the runner, never read from a log.
pub const ENGINE_MISSING: u32 = 127;

/// The mapped Windows error code inside an OS error. Elsewhere raw codes are errno values
/// (5 is I/O error, 53 something unrelated), so only the error kind says what happened.
fn windows_code(error: &io::Error) -> Option<u32> {
    if !cfg!(windows) {
        return None;
    }
    error
        .raw_os_error()
        .and_then(|code| u32::try_from(code).ok())
        .filter(|code| MAPPED_CODES.contains(code))
}

/// The first mapped error in a robocopy log, and the folder it is about. `source` (A) and
/// `destination` (B) tell which one that is. `None` when the log holds no mapped code.
pub fn find_cause(log: &str, source: &str, destination: &str) -> Option<Cause> {
    let (code, line) = first_mapped_error(log)?;
    Some(Cause {
        code,
        folder: folder_of(line, source, destination),
    })
}

const DISK_CONNECTED: &str = "Il disco è collegato?";

/// Why A could not be read before the engine even started.
pub fn explain_source_error(error: &io::Error, source: &str, destination: &str) -> Explanation {
    let code = windows_code(error)
        .or_else(|| (error.kind() == io::ErrorKind::NotFound).then_some(PATH_NOT_FOUND))
        .or_else(|| (error.kind() == io::ErrorKind::PermissionDenied).then_some(ACCESS_DENIED));
    let cause = code.map(|code| Cause {
        code,
        folder: Folder::A,
    });
    describe(cause, source, destination, Moment::Preview)
}

/// Why a new folder could not be created, from what the OS said.
pub fn explain_create_error(error: &io::Error) -> Explanation {
    let (title, advice) = match error.kind() {
        io::ErrorKind::AlreadyExists => (
            "Esiste già una cartella con questo nome.",
            "Scegli un altro nome.",
        ),
        io::ErrorKind::NotFound => return explain_missing_parent(),
        io::ErrorKind::PermissionDenied => (
            "Non ho il permesso di creare una cartella qui.",
            "Scegli un altro posto o controlla i permessi.",
        ),
        io::ErrorKind::ReadOnlyFilesystem => (
            "Il disco è in sola lettura: non posso crearci cartelle.",
            "Scegli un altro disco.",
        ),
        _ => (
            "Non riesco a creare la cartella.",
            "Controlla che il disco o la rete siano collegati e riprova.",
        ),
    };
    Explanation {
        title: title.into(),
        advice: advice.into(),
    }
}

/// The folder a new one should go in is not there (or is not a full path).
pub fn explain_missing_parent() -> Explanation {
    Explanation {
        title: "Non trovo la cartella in cui creare la nuova.".into(),
        advice: DISK_CONNECTED.into(),
    }
}

/// The title and the advice for a failure; a generic one when the cause is unknown.
pub fn describe(
    cause: Option<Cause>,
    source: &str,
    destination: &str,
    moment: Moment,
) -> Explanation {
    let known = cause.and_then(|c| reason(c, source, destination).map(|r| (c, r)));
    let Some((cause, reason)) = known else {
        return match moment {
            Moment::Run => Explanation {
                title: "Non completato: la copia si è fermata per un problema.".into(),
                advice: "Apri il registro per vedere i dettagli.".into(),
            },
            Moment::Preview => Explanation {
                title: "Non riesco a controllare le cartelle.".into(),
                advice: "Controlla che A e B siano raggiungibili e riprova.".into(),
            },
        };
    };
    let title = match moment {
        // "Su questo computer manca rsync." and "Non trovo la cartella A (D:\Foto)." read the
        // same in both.
        _ if cause.code == ENGINE_MISSING => format!("{}.", capitalized(&reason.what)),
        Moment::Preview if matches!(cause.code, 2 | 3) => format!("{}.", capitalized(&reason.what)),
        Moment::Preview => format!("Non riesco a controllare le cartelle: {}.", reason.what),
        Moment::Run => format!("Non completato: {}.", reason.what),
    };
    Explanation {
        title,
        advice: reason.advice,
    }
}

/// The note under the sync's card on the home screen, from how its last Run ended.
pub fn card_note(run: &Run) -> Option<String> {
    match run.outcome {
        Outcome::Ok => None,
        Outcome::Cancelled => Some(STOPPED_NOTE.to_string()),
        Outcome::Error => Some(
            run.cause
                .and_then(|cause| reason(cause, &run.source, &run.destination))
                .map_or_else(
                    || GENERIC_NOTE.to_string(),
                    |r| {
                        r.note
                            .unwrap_or_else(|| format!("{}. {}", capitalized(&r.what), r.advice))
                    },
                ),
        ),
    }
}

/// The result screen's text for a Run that ended in `Error`.
pub fn explain_run(run: &Run) -> Option<Explanation> {
    (run.outcome == Outcome::Error)
        .then(|| describe(run.cause, &run.source, &run.destination, Moment::Run))
}

struct Reason {
    /// A sentence that starts in lower case and has no full stop: "non trovo la cartella A (…)".
    what: String,
    advice: String,
    /// The home card's text when it is not just `what` and `advice`, said about the last time.
    note: Option<String>,
}

fn reason(cause: Cause, source: &str, destination: &str) -> Option<Reason> {
    let (name, path) = match cause.folder {
        Folder::A => ('A', source.trim()),
        Folder::B => ('B', destination.trim()),
    };
    let (what, advice) = match cause.code {
        53 | 67 => {
            let name = unreachable_name(path);
            return Some(Reason {
                what: format!("{name} non è raggiungibile"),
                advice: "Controlla la connessione e riprova.".into(),
                note: Some(format!(
                    "{} non era raggiungibile. Controlla che sia collegato e riprova.",
                    capitalized(&name)
                )),
            });
        }
        2 | 3 => (
            format!("non trovo la cartella {name} ({path})"),
            if path_rules::is_unc(path) {
                "La cartella di rete è raggiungibile?"
            } else {
                DISK_CONNECTED
            },
        ),
        5 => (
            format!("non ho il permesso di usare la cartella {name} ({path})"),
            "Controlla i permessi e riprova.",
        ),
        ENGINE_MISSING => (
            "su questo computer manca rsync".to_string(),
            "Installalo con il gestore dei pacchetti (per esempio: sudo apt install rsync) e riprova.",
        ),
        _ => return None,
    };
    Some(Reason {
        what,
        advice: advice.into(),
        note: None,
    })
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// First log line holding `<n> (0x<hex>)` with a mapped `n`, and that `n`. Any label before
/// the number (`ERROR`, `ERRORE`…) is ignored.
fn first_mapped_error(log: &str) -> Option<(u32, &str)> {
    log.lines().find_map(|line| {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        tokens.windows(2).find_map(|pair| {
            let hex = pair[1].strip_prefix("(0x")?.strip_suffix(')')?;
            let code: u32 = pair[0].parse().ok()?;
            let is_hex = !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit());
            (is_hex && MAPPED_CODES.contains(&code)).then_some((code, line))
        })
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

/// "la cartella di rete \\NAS" or "il disco D:", in lower case.
fn unreachable_name(path: &str) -> String {
    if path_rules::is_unc(path) {
        let server = path
            .trim_start_matches('\\')
            .split('\\')
            .next()
            .unwrap_or("");
        format!(r"la cartella di rete \\{server}")
    } else {
        let tag = if path.starts_with('/') {
            mount_point(path)
        } else {
            path.chars().take(2).collect::<String>().to_uppercase()
        };
        format!("il disco {tag}")
    }
}

/// The disk a macOS or Linux path lives on, as the user sees it: `/Volumes/Backup`,
/// `/media/anna/Backup`, `/mnt/nas`. Anywhere else the whole path says it best.
fn mount_point(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    let depth = match parts.as_slice() {
        ["Volumes" | "mnt", ..] => 2,
        ["media", ..] => 3,
        ["run", "media", ..] => 4,
        _ => return path.trim_end_matches('/').to_string(),
    };
    format!("/{}", parts[..depth.min(parts.len())].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::domain::SyncMode;

    const A: &str = r"D:\Foto";
    const NAS_B: &str = r"\\NAS\Backup";

    #[test]
    fn the_os_error_decides_the_message_of_a_failed_folder_creation() {
        let explain = |kind| explain_create_error(&io::Error::from(kind));
        assert_eq!(
            explain(io::ErrorKind::PermissionDenied).title,
            "Non ho il permesso di creare una cartella qui."
        );
        assert_eq!(
            explain(io::ErrorKind::ReadOnlyFilesystem).title,
            "Il disco è in sola lettura: non posso crearci cartelle."
        );
        assert_eq!(
            explain(io::ErrorKind::TimedOut).title,
            "Non riesco a creare la cartella."
        );
        assert_eq!(explain(io::ErrorKind::NotFound), explain_missing_parent());
    }

    #[test]
    #[cfg(windows)]
    fn a_write_protected_windows_disk_is_read_only() {
        const ERROR_WRITE_PROTECT: i32 = 19;
        let e = explain_create_error(&io::Error::from_raw_os_error(ERROR_WRITE_PROTECT));
        assert_eq!(
            e.title,
            "Il disco è in sola lettura: non posso crearci cartelle."
        );
    }

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

    fn run_text(code: u32, folder: Folder, a: &str, b: &str) -> Explanation {
        describe(Some(Cause { code, folder }), a, b, Moment::Run)
    }

    fn at(code: u32, folder: Folder) -> Option<Cause> {
        Some(Cause { code, folder })
    }

    #[test]
    fn error_53_is_found_by_code_in_any_language() {
        for text in [english(53, r"\\NAS\Backup\"), italian(53, r"\\NAS\Backup\")] {
            assert_eq!(find_cause(&text, A, NAS_B), at(53, Folder::B));
        }
        let e = run_text(53, Folder::B, A, NAS_B);
        assert_eq!(
            e.title,
            r"Non completato: la cartella di rete \\NAS non è raggiungibile."
        );
        assert_eq!(e.advice, "Controlla la connessione e riprova.");
    }

    #[test]
    fn error_67_is_an_unreachable_share_too() {
        assert_eq!(
            find_cause(&italian(67, r"\\NAS\Backup\x\"), A, NAS_B),
            at(67, Folder::B)
        );
        assert_eq!(
            run_text(67, Folder::B, A, NAS_B).title,
            r"Non completato: la cartella di rete \\NAS non è raggiungibile."
        );
    }

    #[test]
    fn an_unreachable_drive_letter_says_disco() {
        let e = run_text(53, Folder::B, A, r"Z:\Backup");
        assert_eq!(e.title, "Non completato: il disco Z: non è raggiungibile.");
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
                assert_eq!(find_cause(&text, A, r"E:\Backup"), at(code, Folder::A));
            }
            let e = run_text(code, Folder::A, A, r"E:\Backup");
            assert_eq!(
                e.title,
                r"Non completato: non trovo la cartella A (D:\Foto)."
            );
            assert_eq!(e.advice, "Il disco è collegato?");
        }
    }

    #[test]
    fn error_3_on_a_network_folder_asks_about_the_network() {
        let e = run_text(3, Folder::B, A, NAS_B);
        assert_eq!(
            e.title,
            r"Non completato: non trovo la cartella B (\\NAS\Backup)."
        );
        assert_eq!(e.advice, "La cartella di rete è raggiungibile?");
    }

    #[test]
    fn error_5_says_access_was_denied_on_that_folder() {
        for text in [
            log(5, "ERROR", "Copying File", r"E:\Backup\2024\a.jpg"),
            log(5, "ERRORE", "Copia file", r"E:\Backup\2024\a.jpg"),
        ] {
            assert_eq!(find_cause(&text, A, r"E:\Backup"), at(5, Folder::B));
        }
        let e = run_text(5, Folder::B, A, r"E:\Backup");
        assert_eq!(
            e.title,
            r"Non completato: non ho il permesso di usare la cartella B (E:\Backup)."
        );
        assert_eq!(e.advice, "Controlla i permessi e riprova.");
    }

    #[test]
    fn any_other_code_or_no_code_has_no_cause_and_reads_generic() {
        for text in [
            english(32, r"E:\Backup\a.jpg"), // sharing violation: not mapped
            "robocopy ha detto qualcosa senza codice\r\n".to_string(),
            String::new(),
        ] {
            assert_eq!(find_cause(&text, A, r"E:\Backup"), None);
        }
        let e = describe(None, A, r"E:\Backup", Moment::Run);
        assert_eq!(
            e.title,
            "Non completato: la copia si è fermata per un problema."
        );
        assert_eq!(e.advice, "Apri il registro per vedere i dettagli.");
    }

    #[test]
    fn a_failed_preview_uses_preview_wording() {
        let e = describe(at(53, Folder::B), A, NAS_B, Moment::Preview);
        assert_eq!(
            e.title,
            r"Non riesco a controllare le cartelle: la cartella di rete \\NAS non è raggiungibile."
        );
        assert_eq!(e.advice, "Controlla la connessione e riprova.");
        let e = describe(at(5, Folder::A), A, NAS_B, Moment::Preview);
        assert_eq!(
            e.title,
            r"Non riesco a controllare le cartelle: non ho il permesso di usare la cartella A (D:\Foto)."
        );
        let e = describe(None, A, NAS_B, Moment::Preview);
        assert_eq!(e.title, "Non riesco a controllare le cartelle.");
        assert!(!e.title.contains("Non completato"));
    }

    #[test]
    fn a_folder_that_is_not_there_reads_the_same_in_a_preview() {
        let e = describe(at(3, Folder::A), A, NAS_B, Moment::Preview);
        assert_eq!(e.title, r"Non trovo la cartella A (D:\Foto).");
        assert_eq!(e.advice, "Il disco è collegato?");
    }

    #[test]
    fn a_source_is_explained_by_the_kind_of_error_on_every_platform() {
        let explain = |kind| explain_source_error(&io::Error::from(kind), A, NAS_B);
        let missing = explain(io::ErrorKind::NotFound);
        assert_eq!(missing.title, r"Non trovo la cartella A (D:\Foto).");
        assert_eq!(missing.advice, "Il disco è collegato?");
        assert_eq!(
            explain(io::ErrorKind::PermissionDenied).title,
            r"Non riesco a controllare le cartelle: non ho il permesso di usare la cartella A (D:\Foto)."
        );
        assert_eq!(
            explain(io::ErrorKind::TimedOut).title,
            "Non riesco a controllare le cartelle."
        );
    }

    #[test]
    fn a_unix_path_names_the_disk_it_lives_on() {
        for (path, disk) in [
            ("/Volumes/Backup/Foto", "/Volumes/Backup"),
            ("/Volumes/Backup", "/Volumes/Backup"),
            ("/media/anna/Backup/Foto/2024", "/media/anna/Backup"),
            ("/run/media/anna/Backup/Foto", "/run/media/anna/Backup"),
            ("/mnt/nas/Foto", "/mnt/nas"),
            ("/home/anna/Foto/", "/home/anna/Foto"),
        ] {
            assert_eq!(mount_point(path), disk, "{path}");
        }
        let e = run_text(53, Folder::B, "/home/anna/Foto", "/Volumes/Backup/Foto");
        assert_eq!(
            e.title,
            "Non completato: il disco /Volumes/Backup non è raggiungibile."
        );
        let e = run_text(3, Folder::B, "/home/anna/Foto", "/Volumes/Backup/Foto");
        assert_eq!(
            e.title,
            "Non completato: non trovo la cartella B (/Volumes/Backup/Foto)."
        );
        assert_eq!(e.advice, "Il disco è collegato?");
    }

    #[test]
    #[cfg(windows)]
    fn a_source_that_cannot_be_read_is_explained_by_its_os_error() {
        let explain = |error: io::Error| explain_source_error(&error, A, NAS_B);
        assert_eq!(
            explain(io::Error::from(io::ErrorKind::NotFound)).title,
            r"Non trovo la cartella A (D:\Foto)."
        );
        assert_eq!(
            explain(io::Error::from_raw_os_error(3)).advice,
            "Il disco è collegato?"
        );
        assert_eq!(
            explain(io::Error::from_raw_os_error(53)).title,
            r"Non riesco a controllare le cartelle: il disco D: non è raggiungibile."
        );
        assert_eq!(
            explain(io::Error::from_raw_os_error(5)).title,
            r"Non riesco a controllare le cartelle: non ho il permesso di usare la cartella A (D:\Foto)."
        );
        assert_eq!(
            explain(io::Error::from_raw_os_error(1117)).title,
            "Non riesco a controllare le cartelle."
        );
    }

    #[test]
    fn the_folder_is_found_by_whole_path_not_by_a_name_that_starts_the_same() {
        // `D:\Foto 2` is a folder of its own: the error is about A, not about B (`D:\Foto`).
        let (a, b) = (r"D:\Foto 2", r"D:\Foto");
        let line = english(5, r"D:\Foto 2\a.jpg");
        assert_eq!(find_cause(&line, a, b), at(5, Folder::A));
        assert_eq!(
            find_cause(&english(5, r"D:\Foto 2\"), a, b),
            at(5, Folder::A)
        );
        assert_eq!(
            find_cause(&english(5, r"d:/foto/a.jpg"), a, b),
            at(5, Folder::B)
        );
        // B is `D:\Foto2`: a sibling by name, not a parent.
        let line = english(5, r"D:\Foto2\a.jpg");
        assert_eq!(find_cause(&line, A, r"D:\Foto2"), at(5, Folder::B));
    }

    #[test]
    fn a_line_without_a_known_path_blames_the_share_then_b() {
        let line = english(53, r"\\ALTRO\x\");
        assert_eq!(find_cause(&line, A, NAS_B), at(53, Folder::B));
        assert_eq!(find_cause(&line, NAS_B, r"E:\Backup"), at(53, Folder::A));
        assert_eq!(
            find_cause(&english(3, "x"), A, r"E:\Backup"),
            at(3, Folder::B)
        );
    }

    #[test]
    fn the_first_mapped_error_wins() {
        let text = english(32, r"E:\a") + &english(5, r"E:\Backup\b") + &english(53, NAS_B);
        assert_eq!(find_cause(&text, A, r"E:\Backup"), at(5, Folder::B));
    }

    fn run(outcome: Outcome, cause: Option<Cause>) -> Run {
        Run {
            sync_id: Some(1),
            sync_name: "Foto".into(),
            source: A.into(),
            destination: NAS_B.into(),
            mode: SyncMode::Copy,
            new: 0,
            updated: 0,
            deleted: 0,
            duration_ms: 1,
            outcome,
            started_at: "2026-11-02T21:30:00+01:00".into(),
            log_path: String::new(),
            partial: None,
            cause,
        }
    }

    #[test]
    fn the_card_note_is_made_from_the_cause() {
        assert_eq!(
            card_note(&run(Outcome::Error, at(53, Folder::B))).unwrap(),
            r"La cartella di rete \\NAS non era raggiungibile. Controlla che sia collegato e riprova."
        );
        assert_eq!(
            card_note(&run(Outcome::Error, at(3, Folder::A))).unwrap(),
            r"Non trovo la cartella A (D:\Foto). Il disco è collegato?"
        );
        assert_eq!(
            card_note(&run(Outcome::Error, at(5, Folder::B))).unwrap(),
            r"Non ho il permesso di usare la cartella B (\\NAS\Backup). Controlla i permessi e riprova."
        );
    }

    #[test]
    fn the_card_note_without_a_cause_is_generic_and_a_stop_or_success_say_their_own() {
        assert_eq!(
            card_note(&run(Outcome::Error, None)).as_deref(),
            Some(GENERIC_NOTE)
        );
        assert_eq!(
            card_note(&run(Outcome::Cancelled, None)).as_deref(),
            Some(STOPPED_NOTE)
        );
        assert_eq!(card_note(&run(Outcome::Ok, None)), None);
    }

    #[test]
    fn only_a_failed_run_has_an_explanation() {
        let failed = explain_run(&run(Outcome::Error, at(53, Folder::B))).unwrap();
        assert_eq!(
            failed.title,
            r"Non completato: la cartella di rete \\NAS non è raggiungibile."
        );
        assert_eq!(explain_run(&run(Outcome::Cancelled, None)), None);
        assert_eq!(explain_run(&run(Outcome::Ok, None)), None);
    }

    #[test]
    fn a_missing_engine_is_said_plainly_in_a_preview_a_run_and_on_the_card() {
        let missing = Some(Cause {
            code: ENGINE_MISSING,
            folder: Folder::A,
        });
        for moment in [Moment::Preview, Moment::Run] {
            let e = describe(missing, A, NAS_B, moment);
            assert_eq!(e.title, "Su questo computer manca rsync.");
            assert_eq!(
                e.advice,
                "Installalo con il gestore dei pacchetti (per esempio: sudo apt install rsync) e riprova."
            );
        }
        assert_eq!(
            card_note(&run(Outcome::Error, missing)).unwrap(),
            "Su questo computer manca rsync. Installalo con il gestore dei pacchetti (per esempio: sudo apt install rsync) e riprova."
        );
    }

    #[test]
    fn a_message_without_advice_is_just_a_title() {
        let e = Explanation::from("La destinazione non può stare dentro la cartella da copiare.");
        assert_eq!(e.advice, "");
    }
}
