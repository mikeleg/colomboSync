// The only place that talks to Rust: Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Counts, Explanation, FolderListing, ListedSyncs, Preview, RunList, RunProgress, RunReport, Sync, SyncMode } from "./types";

/** Anything that is not an `Explanation` (the call itself failed) becomes a title. */
const asExplanation = (e: unknown): Explanation =>
  typeof e === "object" && e !== null && "title" in e ? (e as Explanation) : { title: String(e), advice: "" };

/** Rejects with an `Explanation`. */
export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode }).catch((e: unknown) => {
    throw asExplanation(e);
  });

/** Runs a saved sync; the run is recorded in the history under its id. */
export const runSync = (sync: Sync, counts: Counts) => invoke<RunReport>("run_sync", { sync, counts });

/** Saved syncs, newest first, each with how its latest Run ended. */
export const listSyncs = () => invoke<ListedSyncs>("list_syncs");

export const saveSync = (name: string, source: string, destination: string, mode: SyncMode) =>
  invoke<Sync>("save_sync", { name, source, destination, mode });

/** "Copia di <last folder of A>". */
export const defaultSyncName = (source: string) => invoke<string>("default_sync_name", { source });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });

/** Makes the folder `name` inside `parent` and resolves with its path; rejects with an `Explanation`. */
export const createFolder = (parent: string, name: string) =>
  invoke<string>("create_folder", { parent, name }).catch((e: unknown) => {
    throw asExplanation(e);
  });

/** Rejects (with the Italian message to show) a B that is A itself or inside A. */
export const checkDestination = (source: string, destination: string) =>
  invoke<void>("check_destination", { source, destination });

/** Past runs, newest first, with the totals and per-sync state for the Storico side panel. */
export const listRuns = () => invoke<RunList>("list_runs");

/** Text of a run's robocopy log; rejects when the file is gone. */
export const readRunLog = (path: string) => invoke<string>("read_run_log", { path });

/** Kills robocopy; the pending `runSync` then resolves with a Cancelled run. `exitWhenDone` closes the app after that. */
export const cancelRun = (exitWhenDone = false) => invoke<boolean>("cancel_run", { exitWhenDone });

export const onRunProgress = (handler: (progress: RunProgress) => void) =>
  listen<RunProgress>("run-progress", (event) => handler(event.payload));

/** The user tried to close the window while a run is going; the window stays open. */
export const onCloseRequested = (handler: () => void) => listen("close-requested", () => handler());

/** The app was started as administrator: mapped network drives are then not visible. */
export const isElevated = () => invoke<boolean>("is_elevated");
