// The only place that talks to Rust: Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Counts, FolderListing, Preview, Run, RunProgress, Sync, SyncMode, SyncSummary } from "./types";

export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

/** Runs a saved sync; the run is recorded in the history under its id. */
export const runSync = (sync: Sync, counts: Counts) => invoke<Run>("run_sync", { sync, counts });

/** Saved syncs, newest first, each with how its latest Run ended. */
export const listSyncs = () => invoke<SyncSummary[]>("list_syncs");

export const saveSync = (name: string, source: string, destination: string, mode: SyncMode) =>
  invoke<Sync>("save_sync", { name, source, destination, mode });

/** "Copia di <last folder of A>". */
export const defaultSyncName = (source: string) => invoke<string>("default_sync_name", { source });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });

/** Past runs, newest first. */
export const listRuns = () => invoke<Run[]>("list_runs");

/** Text of a run's robocopy log; rejects when the file is gone. */
export const readRunLog = (path: string) => invoke<string>("read_run_log", { path });

/** Kills robocopy; the pending `runSync` then resolves with a Cancelled run. `exitWhenDone` closes the app after that. */
export const cancelRun = (exitWhenDone = false) => invoke<boolean>("cancel_run", { exitWhenDone });

export const onRunProgress = (handler: (progress: RunProgress) => void) =>
  listen<RunProgress>("run-progress", (event) => handler(event.payload));

/** The user tried to close the window while a run is going; the window stays open. */
export const onCloseRequested = (handler: () => void) => listen("close-requested", () => handler());
