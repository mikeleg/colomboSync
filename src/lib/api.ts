// The only place that talks to Rust: Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Counts, FolderListing, Preview, Run, RunProgress, SyncMode } from "./types";

export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

export const runSync = (source: string, destination: string, mode: SyncMode, counts: Counts) =>
  invoke<Run>("run_sync", { source, destination, mode, counts });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });

/** Rejects (with the Italian message to show) a B that is A itself or inside A. */
export const checkDestination = (source: string, destination: string) =>
  invoke<void>("check_destination", { source, destination });

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
