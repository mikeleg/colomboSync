//! The Italian sentences that vary with data, outcome or platform and are not errors (those are in
//! `error_messages`): numbers, durations, dates, the words of a Run's result, of its progress and
//! of a preview. The UI shows them as they are; none of it is composed in Svelte.

use super::domain::{Counts, Outcome, Run, SyncMode};
use super::error_messages::Explanation;
use super::path_rules;
use chrono::DateTime;
use serde::Serialize;

/// `n.toLocaleString("it-IT")`: dots between the thousands, but only from five digits on (Italian
/// writes 1000 and 12.345).
pub fn format_number(n: usize) -> String {
    let digits = n.to_string();
    if digits.len() < 5 {
        return digits;
    }
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(digit);
    }
    grouped
}

/// "< 1 min" or "N min", rounding half up.
pub fn format_duration(ms: u64) -> String {
    let minutes = (ms as f64 / 60_000.0).round() as u64;
    if minutes < 1 {
        "< 1 min".to_string()
    } else {
        format!("{minutes} min")
    }
}

/// "2026-11-02T21:30:00+01:00" -> "02/11/2026 21:30": the local time as it was stored, without any
/// time zone conversion. What is not such a time is returned as it is.
pub fn format_when(started_at: &str) -> String {
    DateTime::parse_from_rfc3339(started_at).map_or_else(
        |_| started_at.to_string(),
        |t| t.format("%d/%m/%Y %H:%M").to_string(),
    )
}

/// "1 file copiato" / "40 file copiati": the number and the noun that agrees with it.
fn counted(n: usize, singular: &str, plural: &str) -> String {
    format!(
        "{} {}",
        format_number(n),
        if n == 1 { singular } else { plural }
    )
}

/// "1 file copiato (parziale)" / "40 file copiati (parziale)".
pub fn partial_text(n: usize) -> String {
    format!("{} (parziale)", counted(n, "file copiato", "file copiati"))
}

/// Time left, from the `done` files of `total` in `elapsed_ms`.
pub fn eta_text(done: usize, total: usize, elapsed_ms: u64) -> String {
    if elapsed_ms < 1000 || done == 0 {
        return "Calcolo il tempo che manca…".to_string();
    }
    let files_per_second = done as f64 / (elapsed_ms as f64 / 1000.0);
    let seconds = ((total as f64 - done as f64) / files_per_second).ceil();
    if seconds <= 1.0 {
        "Quasi finito.".to_string()
    } else {
        format!("Mancano circa {} secondi.", format_number(seconds as usize))
    }
}

/// The heading of a Run's result: what was done, or why it was not.
pub fn run_title(run: &Run, explanation: Option<&Explanation>) -> String {
    match run.outcome {
        Outcome::Ok => {
            let deleted = if run.mode == SyncMode::Mirror && run.deleted > 0 {
                format!(
                    ", {} da B",
                    counted(run.deleted, "cancellato", "cancellati")
                )
            } else {
                String::new()
            };
            format!(
                "Fatto: {}, {}{deleted} in {}",
                counted(run.new, "file copiato", "file copiati"),
                counted(run.updated, "aggiornato", "aggiornati"),
                format_duration(run.duration_ms)
            )
        }
        Outcome::Cancelled => "Annullato: ho fermato la copia.".to_string(),
        Outcome::Error => {
            explanation.map_or_else(|| "Non completato".to_string(), |e| e.title.clone())
        }
    }
}

/// The one word of the schema panel for how a Run ended.
pub fn run_caption(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Ok => "Fatto.",
        Outcome::Cancelled => "Annullato.",
        Outcome::Error => "Non completato.",
    }
}

/// What the result says about the files copied before a failed or stopped Run ended, when it
/// knows how many.
pub fn run_result_note(run: &Run) -> Option<String> {
    let partial = run.partial?;
    Some(match (partial, run.outcome) {
        (0, _) => "Nessun file è stato copiato.".to_string(),
        (n, Outcome::Cancelled) => format!("{}. I file già copiati restano in B.", partial_text(n)),
        (n, _) => partial_text(n),
    })
}

/// The caption of the schema panel on a preview that is not blocked.
pub fn preview_caption(counts: Counts, mode: SyncMode) -> String {
    if counts.total() == 0 {
        return "B è già aggiornato.".to_string();
    }
    let deleted = if mode == SyncMode::Mirror {
        format!(", cancellarne {} da B", format_number(counts.deleted))
    } else {
        String::new()
    };
    format!(
        "Da fare: copiare {} file nuovi, aggiornarne {}{deleted}.",
        format_number(counts.new),
        format_number(counts.updated)
    )
}

/// A piece of a sentence Rust makes: the screen shows `bold` pieces (the counts) in bold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub text: String,
    pub bold: bool,
}

fn words(text: &str) -> Segment {
    Segment {
        text: text.to_string(),
        bold: false,
    }
}

fn count(n: usize) -> Segment {
    Segment {
        text: format_number(n),
        bold: true,
    }
}

/// The first sentence of a preview that is not blocked, in pieces: each count is bold.
pub fn preview_summary(counts: Counts, mode: SyncMode) -> Vec<Segment> {
    let mut sentence = if mode == SyncMode::Mirror {
        vec![words("Ecco cosa succederà in B: ")]
    } else {
        vec![words("Ecco cosa verrà fatto: ")]
    };
    sentence.extend([
        count(counts.new),
        words(" file nuovi, "),
        count(counts.updated),
        words(" da aggiornare"),
    ]);
    if mode == SyncMode::Mirror {
        sentence.extend([words(", "), count(counts.deleted), words(" da cancellare")]);
    }
    sentence.push(words("."));
    sentence
}

/// What kind of place a path is, for the schema panel and the home cards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub is_network: bool,
    /// "Rete · NAS", "Disco D:" or "Cartella".
    pub label: String,
}

pub fn place(path: &str) -> Place {
    let path = path_rules::strip_verbatim(path);
    if path_rules::is_unc(&path) {
        let server = path[2..].split('\\').next().unwrap_or_default();
        return Place {
            is_network: true,
            label: format!("Rete · {server}"),
        };
    }
    let label = match path.as_bytes() {
        [letter, b':', ..] if letter.is_ascii_alphabetic() => {
            format!("Disco {}:", letter.to_ascii_uppercase() as char)
        }
        _ => "Cartella".to_string(),
    };
    Place {
        is_network: false,
        label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::domain::counts;
    use crate::shared::domain::sample_run;

    // The same inputs and outputs as the helpers of src/shared/format.ts had in Svelte.

    #[test]
    fn numbers_are_written_like_it_it_does() {
        // Values taken from `n.toLocaleString("it-IT")` in Node.
        for (n, text) in [
            (0, "0"),
            (1, "1"),
            (999, "999"),
            (1000, "1000"),
            (9999, "9999"),
            (10_000, "10.000"),
            (12345, "12.345"),
            (100_000, "100.000"),
            (1_234_567, "1.234.567"),
        ] {
            assert_eq!(format_number(n), text);
        }
    }

    #[test]
    fn durations_round_to_the_minute_half_up() {
        assert_eq!(format_duration(0), "< 1 min");
        assert_eq!(format_duration(4_000), "< 1 min");
        assert_eq!(format_duration(29_999), "< 1 min");
        assert_eq!(format_duration(30_000), "1 min");
        assert_eq!(format_duration(61_000), "1 min");
        assert_eq!(format_duration(90_000), "2 min");
        assert_eq!(format_duration(3_600_000), "60 min");
    }

    #[test]
    fn when_shows_the_local_date_and_time_written_in_the_rfc_3339_string() {
        assert_eq!(format_when("2026-11-02T21:30:00+01:00"), "02/11/2026 21:30");
        assert_eq!(
            format_when("2026-11-02T21:30:00.123456+01:00"),
            "02/11/2026 21:30"
        );
        // Whatever the zone: what is stored is what is shown.
        assert_eq!(format_when("2026-11-02T21:30:00Z"), "02/11/2026 21:30");
    }

    #[test]
    fn when_shows_an_unexpected_value_as_it_is() {
        for odd in ["ieri sera", "non è una data", "", "2026-11-02 21:30:00"] {
            assert_eq!(format_when(odd), odd);
        }
    }

    #[test]
    fn partial_text_uses_the_singular() {
        assert_eq!(partial_text(0), "0 file copiati (parziale)");
        assert_eq!(partial_text(1), "1 file copiato (parziale)");
        assert_eq!(partial_text(40), "40 file copiati (parziale)");
        assert_eq!(partial_text(12_345), "12.345 file copiati (parziale)");
    }

    #[test]
    fn eta_is_worked_out_from_the_rate_so_far() {
        const WAIT: &str = "Calcolo il tempo che manca…";
        assert_eq!(eta_text(0, 151, 0), WAIT);
        assert_eq!(eta_text(0, 151, 10_000), WAIT);
        assert_eq!(eta_text(40, 151, 999), WAIT);
        // 4 files/s, 111 left -> 28 s
        assert_eq!(eta_text(40, 151, 10_000), "Mancano circa 28 secondi.");
        assert_eq!(eta_text(150, 151, 30_000), "Quasi finito.");
        assert_eq!(eta_text(151, 151, 30_000), "Quasi finito.");
        assert_eq!(eta_text(1, 10_000, 1_000), "Mancano circa 9999 secondi.");
        assert_eq!(eta_text(1, 100_000, 1_000), "Mancano circa 99.999 secondi.");
    }

    fn run(over: impl FnOnce(&mut Run)) -> Run {
        let mut run = Run {
            new: 12,
            updated: 3,
            duration_ms: 4_000,
            ..sample_run()
        };
        over(&mut run);
        run
    }

    #[test]
    fn a_successful_run_says_what_was_copied_and_updated_and_how_long_it_took() {
        assert_eq!(
            run_title(&run(|_| {}), None),
            "Fatto: 12 file copiati, 3 aggiornati in < 1 min"
        );
        assert_eq!(
            run_title(
                &run(|r| {
                    r.new = 1;
                    r.updated = 1;
                }),
                None
            ),
            "Fatto: 1 file copiato, 1 aggiornato in < 1 min"
        );
    }

    #[test]
    fn a_mirror_run_also_says_how_many_files_were_deleted_from_b() {
        let mirror = |deleted| {
            run_title(
                &run(|r| {
                    r.mode = SyncMode::Mirror;
                    r.deleted = deleted;
                }),
                None,
            )
        };
        assert_eq!(
            mirror(12),
            "Fatto: 12 file copiati, 3 aggiornati, 12 cancellati da B in < 1 min"
        );
        assert_eq!(mirror(0), "Fatto: 12 file copiati, 3 aggiornati in < 1 min");
        assert!(mirror(1).contains(", 1 cancellato da B in "));
        // A copy never deletes from B: nothing to say, whatever is stored.
        let copy = run(|r| r.deleted = 5);
        assert_eq!(
            run_title(&copy, None),
            "Fatto: 12 file copiati, 3 aggiornati in < 1 min"
        );
    }

    #[test]
    fn a_stopped_run_and_a_failed_one_have_their_own_title() {
        let stopped = run(|r| r.outcome = Outcome::Cancelled);
        assert_eq!(run_title(&stopped, None), "Annullato: ho fermato la copia.");

        let failed = run(|r| r.outcome = Outcome::Error);
        let why = Explanation {
            title: "Non completato: la copia si è fermata per un problema.".into(),
            advice: "Apri il registro per vedere i dettagli.".into(),
        };
        assert_eq!(run_title(&failed, Some(&why)), why.title);
        assert_eq!(run_title(&failed, None), "Non completato");
    }

    #[test]
    fn the_schema_caption_follows_the_outcome() {
        assert_eq!(run_caption(Outcome::Ok), "Fatto.");
        assert_eq!(run_caption(Outcome::Cancelled), "Annullato.");
        assert_eq!(run_caption(Outcome::Error), "Non completato.");
    }

    #[test]
    fn the_result_note_says_what_stays_in_b() {
        let note = |outcome, partial| {
            run_result_note(&run(|r| {
                r.outcome = outcome;
                r.partial = partial;
            }))
        };
        assert_eq!(
            note(Outcome::Cancelled, Some(40)).as_deref(),
            Some("40 file copiati (parziale). I file già copiati restano in B.")
        );
        assert_eq!(
            note(Outcome::Cancelled, Some(0)).as_deref(),
            Some("Nessun file è stato copiato.")
        );
        assert_eq!(
            note(Outcome::Error, Some(0)).as_deref(),
            Some("Nessun file è stato copiato.")
        );
        assert_eq!(
            note(Outcome::Error, Some(40)).as_deref(),
            Some("40 file copiati (parziale)")
        );
        assert_eq!(note(Outcome::Ok, None), None);
        assert_eq!(note(Outcome::Cancelled, None), None);
    }

    #[test]
    fn the_preview_caption_says_what_is_to_do_or_that_b_is_up_to_date() {
        assert_eq!(
            preview_caption(counts(0, 0, 0), SyncMode::Copy),
            "B è già aggiornato."
        );
        assert_eq!(
            preview_caption(counts(0, 0, 0), SyncMode::Mirror),
            "B è già aggiornato."
        );
        assert_eq!(
            preview_caption(counts(148, 1, 0), SyncMode::Copy),
            "Da fare: copiare 148 file nuovi, aggiornarne 1."
        );
        assert_eq!(
            preview_caption(counts(3, 1, 2), SyncMode::Mirror),
            "Da fare: copiare 3 file nuovi, aggiornarne 1, cancellarne 2 da B."
        );
        assert_eq!(
            preview_caption(counts(0, 0, 2), SyncMode::Mirror),
            "Da fare: copiare 0 file nuovi, aggiornarne 0, cancellarne 2 da B."
        );
        assert_eq!(
            preview_caption(counts(12_345, 0, 0), SyncMode::Copy),
            "Da fare: copiare 12.345 file nuovi, aggiornarne 0."
        );
    }

    /// The sentence as one string, and the pieces that are bold.
    fn sentence(segments: &[Segment]) -> (String, Vec<&str>) {
        let text = segments.iter().map(|s| s.text.as_str()).collect();
        let bold = segments
            .iter()
            .filter(|s| s.bold)
            .map(|s| s.text.as_str())
            .collect();
        (text, bold)
    }

    #[test]
    fn the_preview_summary_also_counts_what_will_be_deleted_in_a_mirror() {
        assert_eq!(
            sentence(&preview_summary(counts(148, 1, 0), SyncMode::Copy)),
            (
                "Ecco cosa verrà fatto: 148 file nuovi, 1 da aggiornare.".to_string(),
                vec!["148", "1"]
            )
        );
        assert_eq!(
            sentence(&preview_summary(counts(3, 1, 2), SyncMode::Mirror)),
            (
                "Ecco cosa succederà in B: 3 file nuovi, 1 da aggiornare, 2 da cancellare."
                    .to_string(),
                vec!["3", "1", "2"]
            )
        );
    }

    #[test]
    fn the_preview_summary_bolds_the_counts_and_only_them() {
        let segments = preview_summary(counts(12_345, 0, 0), SyncMode::Copy);
        assert_eq!(
            segments.iter().map(|s| s.bold).collect::<Vec<_>>(),
            [false, true, false, true, false, false]
        );
        assert_eq!(segments[1].text, "12.345");
    }

    #[test]
    fn a_place_is_a_share_a_drive_or_a_folder() {
        let place = |path| {
            let p = place(path);
            (p.is_network, p.label)
        };
        assert_eq!(place(r"\\NAS\Backup\Foto"), (true, "Rete · NAS".into()));
        assert_eq!(place(r"\\NAS"), (true, "Rete · NAS".into()));
        assert_eq!(place(r"D:\Foto"), (false, "Disco D:".into()));
        assert_eq!(place(r"d:\foto"), (false, "Disco D:".into()));
        assert_eq!(place("C:"), (false, "Disco C:".into()));
        // What a Mac or Linux path looks like: no drive, no share.
        assert_eq!(place("/Volumes/Backup"), (false, "Cartella".into()));
        assert_eq!(place("/home/anna/Foto"), (false, "Cartella".into()));
        assert_eq!(place(""), (false, "Cartella".into()));
        assert_eq!(place(r"\Foto"), (false, "Cartella".into()));
    }

    #[test]
    fn a_verbatim_path_is_shown_as_its_disk_or_share() {
        let place = |path| {
            let p = place(path);
            (p.is_network, p.label)
        };
        assert_eq!(place(r"\\?\D:\Foto"), (false, "Disco D:".into()));
        assert_eq!(place(r"\\?\UNC\NAS\Backup"), (true, "Rete · NAS".into()));
        assert_eq!(place(r"\\?\unc\NAS\Backup"), (true, "Rete · NAS".into()));
    }

    #[test]
    fn a_volume_verbatim_path_is_a_network_place_as_before() {
        assert!(place(r"\\?\Volume{1234-abcd}\Backup").is_network);
    }
}
