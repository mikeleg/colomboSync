//! Why a copy failed, in plain Italian. A `Cause` (a mapped Windows error code and the folder it is
//! about) comes from the engine or from an OS error; this module turns it into the text the user
//! reads. Only the cause is kept; the texts are made from it when they are needed.

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

/// The Windows error codes that have a message; engines may only produce these (plus `ENGINE_MISSING`).
pub const MAPPED_CODES: [u32; 5] = [2, 3, 5, 53, 67];
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

/// What the folder picker says when a folder cannot be opened (or could not be listed at all: the
/// background task died, not the disk).
pub fn cannot_open_folder() -> Explanation {
    "Non riesco ad aprire questa cartella. Il disco o la rete sono collegati?".into()
}

/// Avanti on the step "Da dove" with no folder chosen.
pub fn choose_source() -> Explanation {
    "Scegli prima la cartella da copiare.".into()
}

/// Avanti on the step "Dove" with no folder chosen.
pub fn choose_destination() -> Explanation {
    "Scegli prima dove mettere la copia.".into()
}

/// Avanti on the step "Nome" with no name written.
pub fn name_missing() -> Explanation {
    "Scrivi un nome per riconoscere questa sincronizzazione.".into()
}

fn explanation(title: &str, advice: &str) -> Explanation {
    Explanation {
        title: title.into(),
        advice: advice.into(),
    }
}

const REDO_PREVIEW: &str = "Controlla di nuovo le cartelle con l’anteprima, poi avvia.";

/// A run was asked for with no preview token at all.
pub fn run_without_preview() -> Explanation {
    explanation(
        "Non ho avviato la copia: manca l’anteprima.",
        "Prima controlla le due cartelle con l’anteprima, poi avvia.",
    )
}

/// The token is not the one of the last preview: it was never given, it was replaced by a newer
/// preview, or it already confirmed a run.
pub fn run_with_stale_preview() -> Explanation {
    explanation(
        "Non ho avviato la copia: l’anteprima non è più valida.",
        REDO_PREVIEW,
    )
}

/// The preview is of other folders, or of the other mode, than the sync that was asked to run.
pub fn run_with_other_preview() -> Explanation {
    explanation(
        "Non ho avviato la copia: l’anteprima è di altre cartelle o di un’altra modalità.",
        REDO_PREVIEW,
    )
}

/// A mirror run that would delete files from B, without the user's confirmation.
pub fn mirror_without_consent() -> Explanation {
    explanation(
        "Non ho avviato la copia: manca la tua conferma.",
        "Con lo specchio i file che ci sono solo in B vengono cancellati. Conferma di aver capito e riprova.",
    )
}

/// A background task died instead of answering: nothing the user did.
pub fn unexpected_failure() -> Explanation {
    explanation(
        "Qualcosa è andato storto dentro l’app.",
        "Riprova. Se succede ancora, chiudi e riapri ColomboSync.",
    )
}

/// `syncs.json` can be read neither nor set aside.
pub fn cannot_read_syncs() -> Explanation {
    "Non riesco a leggere le sincronizzazioni salvate.".into()
}

/// Fixed words: what the OS said is for the developer, not for the user.
pub fn cannot_save_sync() -> Explanation {
    explanation(
        "Non riesco a salvare la sincronizzazione.",
        "Controlla lo spazio sul disco e i permessi, poi riprova.",
    )
}

pub fn cannot_read_history() -> Explanation {
    "Non riesco a leggere lo storico.".into()
}

/// The path asked for is not inside the folder of the logs: nothing the user can fix.
pub fn not_a_log() -> Explanation {
    "Questo file non è un registro di ColomboSync.".into()
}

/// A run's log could not be shown: the file is gone, or the OS could not read it.
pub fn explain_log_error(error: &io::Error) -> Explanation {
    if error.kind() == io::ErrorKind::NotFound {
        return "Il registro di questa copia non è più disponibile.".into();
    }
    explanation(
        "Non riesco ad aprire il registro di questa copia.",
        "Controlla che il file ci sia ancora e di avere il permesso di leggerlo.",
    )
}

pub fn run_already_in_progress() -> Explanation {
    "C’è già una copia in corso.".into()
}

/// The folder for the logs could not be made.
pub fn cannot_prepare_logs() -> Explanation {
    explanation(
        "Non riesco a preparare la cartella dei registri.",
        "Controlla lo spazio sul disco e i permessi, poi riprova.",
    )
}

/// The Run is over, but its line could not be added to the history.
pub fn cannot_write_history() -> Explanation {
    explanation(
        "Non riesco a salvare questa copia nello storico.",
        "Controlla lo spazio sul disco e i permessi. La copia potrebbe essere già stata fatta: guarda in B.",
    )
}

/// The engine was started but how it ended could not be read.
pub fn cannot_follow_engine() -> Explanation {
    explanation(
        "Non riesco a sapere come è finita la copia.",
        "Guarda cosa c’è in B e riprova.",
    )
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
    use crate::shared::domain::sample_run;

    const A: &str = r"D:\Foto";
    const NAS_B: &str = r"\\NAS\Backup";

    #[test]
    fn the_commands_words_keep_what_the_ui_used_to_say() {
        assert_eq!(
            cannot_read_syncs().title,
            "Non riesco a leggere le sincronizzazioni salvate."
        );
        assert_eq!(
            cannot_save_sync().title,
            "Non riesco a salvare la sincronizzazione."
        );
        assert_eq!(
            cannot_read_history().title,
            "Non riesco a leggere lo storico."
        );
        assert_eq!(
            run_already_in_progress().title,
            "C’è già una copia in corso."
        );
        assert_eq!(
            cannot_open_folder().title,
            "Non riesco ad aprire questa cartella. Il disco o la rete sono collegati?"
        );
        assert_eq!(cannot_open_folder().advice, "");
        assert_eq!(
            run_without_preview().advice,
            "Prima controlla le due cartelle con l’anteprima, poi avvia."
        );
    }

    #[test]
    fn a_log_that_cannot_be_shown_says_if_it_is_gone_or_unreadable_and_never_the_os_text() {
        let gone = explain_log_error(&io::Error::from(io::ErrorKind::NotFound));
        assert_eq!(
            gone.title,
            "Il registro di questa copia non è più disponibile."
        );
        assert_eq!(gone.advice, "");

        let denied = io::Error::new(io::ErrorKind::PermissionDenied, "os says no");
        for error in [denied, io::Error::from(io::ErrorKind::TimedOut)] {
            let unreadable = explain_log_error(&error);
            assert_eq!(
                unreadable.title,
                "Non riesco ad aprire il registro di questa copia."
            );
            assert!(!unreadable.advice.contains("os says no"));
        }
    }

    #[test]
    fn a_path_outside_the_logs_folder_has_its_own_words_and_no_advice_to_follow() {
        let outside = not_a_log();
        assert_eq!(
            outside.title,
            "Questo file non è un registro di ColomboSync."
        );
        assert_eq!(outside.advice, "");
        assert_ne!(
            outside,
            explain_log_error(&io::Error::from(io::ErrorKind::PermissionDenied))
        );
    }

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

    fn run_text(code: u32, folder: Folder, a: &str, b: &str) -> Explanation {
        describe(Some(Cause { code, folder }), a, b, Moment::Run)
    }

    fn at(code: u32, folder: Folder) -> Option<Cause> {
        Some(Cause { code, folder })
    }

    #[test]
    fn error_53_says_the_share_is_unreachable() {
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
        let e = run_text(5, Folder::B, A, r"E:\Backup");
        assert_eq!(
            e.title,
            r"Non completato: non ho il permesso di usare la cartella B (E:\Backup)."
        );
        assert_eq!(e.advice, "Controlla i permessi e riprova.");
    }

    #[test]
    fn a_run_without_a_cause_reads_generic() {
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

    fn run(outcome: Outcome, cause: Option<Cause>) -> Run {
        Run {
            sync_id: Some(1),
            source: A.into(),
            destination: NAS_B.into(),
            outcome,
            cause,
            ..sample_run()
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
