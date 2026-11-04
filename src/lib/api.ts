// The only place that talks to Rust: Tauri commands and the folder dialog.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Counts, Preview, Run, RunProgress, SyncMode } from "./types";

export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

export const runSync = (source: string, destination: string, mode: SyncMode, counts: Counts) =>
  invoke<Run>("run_sync", { source, destination, mode, counts });

/** Folder chosen in the system dialog, or null if the user cancelled. */
export async function chooseFolder(): Promise<string | null> {
  const chosen = await open({ directory: true });
  return typeof chosen === "string" ? chosen : null;
}

/** Kills robocopy; the pending `runSync` then resolves with a Cancelled run. `exitWhenDone` closes the app after that. */
export const cancelRun = (exitWhenDone = false) => invoke<boolean>("cancel_run", { exitWhenDone });

export const onRunProgress = (handler: (progress: RunProgress) => void) =>
  listen<RunProgress>("run-progress", (event) => handler(event.payload));

/** The user tried to close the window while a run is going; the window stays open. */
export const onCloseRequested = (handler: () => void) => listen("close-requested", () => handler());
