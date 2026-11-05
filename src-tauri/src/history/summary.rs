//! The numbers beside the Storico: totals over `history.jsonl` and the state of each sync.

use crate::history::{self, Outcome, Run};
use crate::syncs::sync_store::SavedSync;
use chrono::DateTime;
use serde::Serialize;

#[derive(Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySummary {
    pub runs: usize,
    pub ok: usize,
    pub failed: usize,
    pub cancelled: usize,
    /// New + updated files of the Ok runs, plus what failed or stopped runs had copied.
    pub files_copied: usize,
    pub files_deleted: usize,
    /// One per sync found in the history: those with a warning first, then by name.
    pub syncs: Vec<SyncStatus>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub id: u64,
    /// Its current name in `syncs.json`; the name it had in its latest Run if it is gone from there.
    pub name: String,
    /// Start time (RFC 3339, as stored) of its latest Ok run.
    pub last_success: Option<String>,
    /// Set when its latest run ended in `Error`.
    pub warning: Option<String>,
}

/// `runs` newest first, as `history::read` returns them. Runs without a sync id (written before
/// syncs had ids) count in the totals but belong to no sync.
pub fn summarize(runs: &[Run], syncs: &[SavedSync]) -> HistorySummary {
    let mut summary = HistorySummary {
        runs: runs.len(),
        ..HistorySummary::default()
    };
    for run in runs {
        match run.outcome {
            Outcome::Ok => summary.ok += 1,
            Outcome::Error => summary.failed += 1,
            Outcome::Cancelled => summary.cancelled += 1,
        }
        summary.files_copied += run.partial.unwrap_or(run.new + run.updated);
        summary.files_deleted += run.deleted;
    }
    let mut statuses: Vec<SyncStatus> = history::latest_per_sync(runs)
        .into_iter()
        .map(|(id, latest)| SyncStatus {
            id,
            name: syncs
                .iter()
                .find(|s| s.id == id)
                .map_or(&latest.sync_name, |s| &s.name)
                .clone(),
            last_success: runs
                .iter()
                .find(|r| r.sync_id == Some(id) && r.outcome == Outcome::Ok)
                .map(|r| r.started_at.clone()),
            warning: (latest.outcome == Outcome::Error).then(|| {
                format!(
                    "L’ultima copia è finita con un errore il {}.",
                    when(&latest.started_at)
                )
            }),
        })
        .collect();
    statuses.sort_by_cached_key(|s| (s.warning.is_none(), s.name.to_lowercase()));
    summary.syncs = statuses;
    summary
}

/// "2026-11-02T21:30:00+01:00" -> "02/11/2026 21:30": the local time as it was stored.
fn when(started_at: &str) -> String {
    DateTime::parse_from_rfc3339(started_at).map_or_else(
        |_| started_at.to_string(),
        |t| t.format("%d/%m/%Y %H:%M").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::SyncMode;

    fn saved(id: u64, name: &str) -> SavedSync {
        SavedSync {
            id,
            name: name.into(),
            source: r"D:\Foto".into(),
            destination: r"E:\Backup".into(),
            mode: SyncMode::Copy,
        }
    }

    fn run(sync_id: Option<u64>, outcome: Outcome, started_at: &str) -> Run {
        Run {
            sync_id,
            sync_name: "Nome nello storico".into(),
            source: r"D:\Foto".into(),
            destination: r"E:\Backup".into(),
            mode: SyncMode::Copy,
            new: 10,
            updated: 2,
            deleted: 0,
            duration_ms: 1,
            outcome,
            started_at: started_at.into(),
            log_path: String::new(),
            partial: None,
            cause: None,
        }
    }

    #[test]
    fn empty_history_is_all_zeros_and_no_syncs() {
        assert_eq!(
            summarize(&[], &[saved(1, "Foto")]),
            HistorySummary::default()
        );
    }

    #[test]
    fn counts_runs_by_outcome_and_adds_up_files() {
        let mirror = Run {
            mode: SyncMode::Mirror,
            deleted: 5,
            ..run(Some(1), Outcome::Ok, "2026-11-04T10:00:00+01:00")
        };
        // Failed and stopped runs keep no new/updated, only the partial count (none in old lines).
        let failed = Run {
            new: 0,
            updated: 0,
            partial: Some(7),
            ..run(Some(1), Outcome::Error, "2026-11-03T10:00:00+01:00")
        };
        let stopped = Run {
            new: 0,
            updated: 0,
            partial: Some(3),
            ..run(Some(2), Outcome::Cancelled, "2026-11-02T10:00:00+01:00")
        };
        let old_failed = Run {
            new: 0,
            updated: 0,
            ..run(None, Outcome::Error, "2026-11-01T10:00:00+01:00")
        };
        let ok = run(Some(2), Outcome::Ok, "2026-10-31T10:00:00+01:00");

        let summary = summarize(&[mirror, failed, stopped, old_failed, ok], &[]);

        assert_eq!(
            (summary.runs, summary.ok, summary.failed, summary.cancelled),
            (5, 2, 2, 1)
        );
        assert_eq!(summary.files_copied, 12 + 7 + 3 + 12);
        assert_eq!(summary.files_deleted, 5);
    }

    #[test]
    fn each_sync_shows_its_current_name_last_success_and_a_warning_when_it_last_failed() {
        let runs = [
            run(Some(1), Outcome::Error, "2026-11-04T10:00:00+01:00"),
            run(Some(1), Outcome::Ok, "2026-11-02T21:30:00+01:00"),
            run(Some(1), Outcome::Ok, "2026-11-01T09:00:00+01:00"),
            run(Some(2), Outcome::Ok, "2026-11-03T08:05:00+01:00"),
            run(Some(2), Outcome::Cancelled, "2026-11-02T08:05:00+01:00"),
        ];

        let summary = summarize(&runs, &[saved(1, "Documenti"), saved(2, "Foto")]);

        assert_eq!(
            summary.syncs,
            [
                SyncStatus {
                    id: 1,
                    name: "Documenti".into(),
                    last_success: Some("2026-11-02T21:30:00+01:00".into()),
                    warning: Some(
                        "L’ultima copia è finita con un errore il 04/11/2026 10:00.".into()
                    ),
                },
                SyncStatus {
                    id: 2,
                    name: "Foto".into(),
                    last_success: Some("2026-11-03T08:05:00+01:00".into()),
                    warning: None,
                },
            ]
        );
    }

    #[test]
    fn a_sync_that_never_succeeded_has_no_last_success_and_a_gone_sync_keeps_its_old_name() {
        let runs = [run(Some(9), Outcome::Error, "2026-11-04T10:00:00+01:00")];

        let summary = summarize(&runs, &[]);

        assert_eq!(summary.syncs[0].name, "Nome nello storico");
        assert_eq!(summary.syncs[0].last_success, None);
        assert!(summary.syncs[0].warning.is_some());
    }

    #[test]
    fn syncs_with_a_warning_come_first_then_by_name() {
        let runs = [
            run(Some(1), Outcome::Ok, "2026-11-04T10:00:00+01:00"),
            run(Some(2), Outcome::Ok, "2026-11-04T10:00:00+01:00"),
            run(Some(3), Outcome::Error, "2026-11-04T10:00:00+01:00"),
            run(None, Outcome::Ok, "2026-11-04T10:00:00+01:00"),
        ];
        let syncs = [saved(1, "Zeta"), saved(2, "alfa"), saved(3, "Mario")];

        let names: Vec<_> = summarize(&runs, &syncs)
            .syncs
            .into_iter()
            .map(|s| s.name)
            .collect();

        assert_eq!(names, ["Mario", "alfa", "Zeta"]);
    }
}
