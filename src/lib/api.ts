// The only place that talks to Rust: Tauri commands and the folder dialog.
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Counts, Preview, Run, SyncMode } from "./types";

export const previewSync = (source: string, destination: string, mode: SyncMode) =>
  invoke<Preview>("preview_sync", { source, destination, mode });

export const runSync = (source: string, destination: string, mode: SyncMode, counts: Counts) =>
  invoke<Run>("run_sync", { source, destination, mode, counts });

/** Past runs, newest first. */
export const listRuns = () => invoke<Run[]>("list_runs");

/** Text of a run's robocopy log; rejects when the file is gone. */
export const readRunLog = (path: string) => invoke<string>("read_run_log", { path });

/** Folder chosen in the system dialog, or null if the user cancelled. */
export async function chooseFolder(): Promise<string | null> {
  const chosen = await open({ directory: true });
  return typeof chosen === "string" ? chosen : null;
}
