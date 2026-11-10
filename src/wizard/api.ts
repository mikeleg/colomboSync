// The only place the wizard talks to Rust: Tauri commands and events. Each command rejects with an `Explanation`.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { RunReport, Sync, SyncMode } from "../shared/types";
import type { CheckedStepId, FolderListing, Preview, RunProgress } from "./types";

/** The preview, with the token that lets `runSync` confirm it. */
export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

/** Drives when `path` is empty, otherwise the subfolders of `path`. */
export const browseFolders = (path: string) => invoke<FolderListing>("browse_folders", { path });

/**
 * Makes the folder `name` inside `parent` as a B for the folder A `source` and resolves with its
 * path; rejects also when the new folder would be inside A.
 */
export const createFolder = (source: string, parent: string, name: string) =>
  invoke<string>("create_folder", { source, parent, name });

/**
 * Avanti: rejects with what is missing on `step` (a folder to copy, a place for the copy, a name),
 * or with why B cannot be that folder (A itself, inside A, above A).
 */
export const checkStep = (step: CheckedStepId, source: string, destination: string, name: string) =>
  invoke<void>("check_step", { step, source, destination, name });

/**
 * Runs a saved sync as the preview whose `previewToken` Rust gave confirms it; the run is recorded in
 * the history under its id. `consent`: the user understood that a mirror deletes files from B.
 * Rejects with an `Explanation` when Rust refuses (no or old preview, other folders, no consent).
 */
export const runSync = (sync: Sync, previewToken: string | null, consent: boolean) =>
  invoke<RunReport>("run_sync", { sync, previewToken, consent });

/** Stops the copy engine; the pending `runSync` then resolves with a Cancelled run. `exitWhenDone` closes the app after that. */
export const cancelRun = (exitWhenDone = false) => invoke<boolean>("cancel_run", { exitWhenDone });

export const onRunProgress = (handler: (progress: RunProgress) => void) =>
  listen<RunProgress>("run-progress", (event) => handler(event.payload));

/** The user tried to close the window while a run is going; the window stays open. */
export const onCloseRequested = (handler: () => void) => listen("close-requested", () => handler());
