// The run screens talk to Rust through these Tauri commands and events.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { RunReport, Sync } from "../shared/types";
import type { RunProgress } from "./types";

/**
 * Runs a saved sync as the preview whose `previewToken` Rust gave confirms it; the run is recorded in
 * the history under its id. `consent`: the user understood that a mirror deletes files from B.
 * Rejects with an `Explanation` when Rust refuses (no or old preview, other folders, no consent).
 */
export const runSync = (sync: Sync, previewToken: string | null, consent: boolean) =>
  invoke<RunReport>("run_sync", { sync, previewToken, consent });

/** Kills robocopy; the pending `runSync` then resolves with a Cancelled run. `exitWhenDone` closes the app after that. */
export const cancelRun = (exitWhenDone = false) => invoke<boolean>("cancel_run", { exitWhenDone });

export const onRunProgress = (handler: (progress: RunProgress) => void) =>
  listen<RunProgress>("run-progress", (event) => handler(event.payload));

/** The user tried to close the window while a run is going; the window stays open. */
export const onCloseRequested = (handler: () => void) => listen("close-requested", () => handler());
