//! Thin `#[tauri::command]` wrappers for the wizard: the engine runs on a blocking thread so the UI
//! never freezes.

use super::folder_browser::{self, FolderListing};
use super::preview;
use crate::engine::preview_parser::Preview;
use crate::shared::blocking::blocking;
use crate::shared::copy_text::{self, Place, Segment};
use crate::shared::domain::SyncMode;
use crate::shared::error_messages::{self, Explanation};
use crate::shared::path_rules;
use crate::shared::preview_memo::PreviewMemo;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

/// What kind of place a folder path is, for the schema panel next to the wizard.
#[tauri::command]
pub fn describe_place(path: String) -> Place {
    copy_text::place(&path)
}

/// The steps of the wizard that can refuse Avanti, in the words of the UI (`STEPS` in Wizard.svelte).
/// Opzioni never refuses and Anteprima has no Avanti.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    Source,
    Destination,
    Name,
}

/// Avanti: whether the step has what it needs, as the error to show when it has not. `source`,
/// `destination` and `name` are what the user has written so far. On "Dove" B must also be a
/// folder unrelated to A (not A itself, not inside it, not above it).
#[tauri::command]
pub fn check_step(
    step: Step,
    source: String,
    destination: String,
    name: String,
) -> Result<(), Explanation> {
    let missing = match step {
        Step::Source => source.trim().is_empty().then(error_messages::choose_source),
        Step::Destination if destination.trim().is_empty() => {
            Some(error_messages::choose_destination())
        }
        Step::Destination => path_rules::check_destination(&source, &destination)
            .err()
            .map(Explanation::from),
        Step::Name => name.trim().is_empty().then(error_messages::name_missing),
    };
    missing.map_or(Ok(()), Err)
}

/// A preview and the token that lets `run_sync` confirm it; no token when the preview is blocked.
/// With the sentences the screen shows for it: the schema caption (what MirrorGuard says when
/// blocked) and the summary of what would be done, in segments (the counts are bold).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReport {
    #[serde(flatten)]
    preview: Preview,
    preview_token: Option<String>,
    caption: String,
    summary: Vec<Segment>,
}

impl PreviewReport {
    fn new(preview: Preview, preview_token: Option<String>, mode: SyncMode) -> Self {
        let counts = preview.counts();
        PreviewReport {
            caption: preview
                .blocked
                .clone()
                .unwrap_or_else(|| copy_text::preview_caption(counts, mode)),
            summary: copy_text::preview_summary(counts, mode),
            preview,
            preview_token,
        }
    }
}

#[tauri::command]
pub async fn preview_sync(
    app: AppHandle,
    source: String,
    destination: String,
    mode: SyncMode,
) -> Result<PreviewReport, Explanation> {
    // From here no earlier token can start a run, even if this preview fails or ends after a newer one.
    let started = app.state::<PreviewMemo>().begin();
    let preview = blocking({
        let (source, destination) = (source.clone(), destination.clone());
        move || preview::preview(&source, &destination, mode)
    })
    .await?;
    let preview_token =
        app.state::<PreviewMemo>()
            .remember(started, &source, &destination, mode, &preview);
    Ok(PreviewReport::new(preview, preview_token, mode))
}

/// Drives (empty `path`) or the subfolders of `path`. On a blocking thread: a disconnected
/// mapped drive can take seconds to answer and must not freeze the UI.
#[tauri::command]
pub async fn browse_folders(path: String) -> Result<FolderListing, Explanation> {
    blocking(move || Ok(folder_browser::browse(&path)))
        .await
        .map_err(|_| error_messages::cannot_open_folder())
}

/// A new folder `name` inside `parent` to be used as B for the folder A `source`, whose path is
/// returned. Blocking thread, like browsing.
#[tauri::command]
pub async fn create_folder(
    source: String,
    parent: String,
    name: String,
) -> Result<String, Explanation> {
    blocking(move || folder_browser::create_folder(&source, &parent, &name)).await
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::engine::preview_parser::FileGroup;

    fn report(preview: Preview, preview_token: Option<&str>, mode: SyncMode) -> serde_json::Value {
        let preview_token = preview_token.map(str::to_string);
        serde_json::to_value(PreviewReport::new(preview, preview_token, mode)).unwrap()
    }

    fn counted(new: usize, updated: usize, deleted: usize) -> Preview {
        let group = |count| FileGroup {
            count,
            files: Vec::new(),
        };
        Preview {
            new: group(new),
            updated: group(updated),
            deleted: group(deleted),
            ..Preview::default()
        }
    }

    #[test]
    fn the_preview_goes_to_the_ui_with_its_token_next_to_the_counted_groups() {
        let json = report(Preview::default(), Some("1-abc"), SyncMode::Copy);
        assert_eq!(json["previewToken"], "1-abc");
        assert_eq!(json["new"]["count"], 0);
        assert!(json["blocked"].is_null());

        let json = report(Preview::default(), None, SyncMode::Copy);
        assert!(json["previewToken"].is_null());
    }

    #[test]
    fn the_preview_goes_to_the_ui_with_its_caption_and_summary() {
        let json = report(counted(148, 1, 0), None, SyncMode::Copy);
        assert_eq!(
            json["caption"],
            "Da fare: copiare 148 file nuovi, aggiornarne 1."
        );
        assert_eq!(
            json["summary"],
            serde_json::json!([
                { "text": "Ecco cosa verrà fatto: ", "bold": false },
                { "text": "148", "bold": true },
                { "text": " file nuovi, ", "bold": false },
                { "text": "1", "bold": true },
                { "text": " da aggiornare", "bold": false },
                { "text": ".", "bold": false },
            ])
        );

        let json = report(counted(3, 1, 2), None, SyncMode::Mirror);
        assert_eq!(
            json["caption"],
            "Da fare: copiare 3 file nuovi, aggiornarne 1, cancellarne 2 da B."
        );
        assert_eq!(
            json["summary"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["text"].as_str().unwrap())
                .collect::<String>(),
            "Ecco cosa succederà in B: 3 file nuovi, 1 da aggiornare, 2 da cancellare."
        );

        let json = report(counted(0, 0, 0), None, SyncMode::Copy);
        assert_eq!(json["caption"], "B è già aggiornato.");
    }

    #[test]
    fn a_blocked_preview_has_the_reason_for_caption() {
        let blocked = Preview {
            blocked: Some("Motivo deciso da Rust: non si parte.".into()),
            ..counted(0, 0, 9)
        };
        let json = report(blocked, None, SyncMode::Mirror);
        assert_eq!(json["caption"], "Motivo deciso da Rust: non si parte.");
        assert_eq!(json["blocked"], "Motivo deciso da Rust: non si parte.");
    }

    #[test]
    fn a_place_goes_to_the_ui_as_a_flag_and_a_label() {
        let json = serde_json::to_value(describe_place(r"\\NAS\Backup".into())).unwrap();
        assert_eq!(json["isNetwork"], true);
        assert_eq!(json["label"], "Rete · NAS");
    }

    fn check(step: Step, source: &str, destination: &str, name: &str) -> Result<(), Explanation> {
        check_step(step, source.into(), destination.into(), name.into())
    }

    #[test]
    fn avanti_wants_what_the_step_asks_for() {
        assert_eq!(
            check(Step::Source, "  ", r"E:\B", "n"),
            Err("Scegli prima la cartella da copiare.".into())
        );
        assert_eq!(
            check(Step::Destination, r"D:\A", "", "n"),
            Err("Scegli prima dove mettere la copia.".into())
        );
        assert_eq!(
            check(Step::Name, r"D:\A", r"E:\B", " \t"),
            Err("Scrivi un nome per riconoscere questa sincronizzazione.".into())
        );
    }

    #[test]
    fn avanti_asks_only_about_its_own_step() {
        assert_eq!(check(Step::Source, r"D:\A", "", ""), Ok(()));
        assert_eq!(check(Step::Destination, "/A", "/B", ""), Ok(()));
        assert_eq!(check(Step::Name, "", "", "Foto"), Ok(()));
    }

    #[test]
    fn avanti_on_dove_refuses_a_destination_related_to_a() {
        let refused = |source, destination| check(Step::Destination, source, destination, "n");
        assert_eq!(
            refused("/Foto", "/Foto"),
            Err(path_rules::SAME_FOLDER.into())
        );
        assert_eq!(
            refused("/Foto", "/Foto/Copia"),
            Err(path_rules::INSIDE_SOURCE.into())
        );
        assert_eq!(
            refused("/Foto/2024", "/Foto"),
            Err(path_rules::SOURCE_INSIDE_DESTINATION.into())
        );
    }

    #[test]
    fn the_steps_are_named_as_the_ui_names_them() {
        for (id, step) in [
            ("source", Step::Source),
            ("destination", Step::Destination),
            ("name", Step::Name),
        ] {
            let parsed: Step = serde_json::from_value(id.into()).unwrap();
            assert_eq!(parsed, step);
        }
    }
}
