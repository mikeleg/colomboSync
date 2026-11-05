// The history screens talk to Rust through these Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type { RunList } from "./types";

/** Past runs, newest first, with the totals and per-sync state for the Storico side panel. */
export const listRuns = () => invoke<RunList>("list_runs");

/** Text of a run's robocopy log; rejects when the file is gone. */
export const readRunLog = (path: string) => invoke<string>("read_run_log", { path });
