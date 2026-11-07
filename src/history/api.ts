// The history screens talk to Rust through these Tauri commands.
import { invoke } from "@tauri-apps/api/core";
import type { RunList } from "./types";

/** Past runs, newest first, with the totals and per-sync state for the Storico side panel. Rejects with an `Explanation`. */
export const listRuns = () => invoke<RunList>("list_runs");
