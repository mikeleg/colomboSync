// The run screens talk to Rust through these Tauri commands and events.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Counts, RunReport, Sync } from "../shared/types";
import type { RunProgress } from "./types";

/** Runs a saved sync; the run is recorded in the history under its id. */
export const runSync = (sync: Sync, counts: Counts) => invoke<RunReport>("run_sync", { sync, counts });

/** Kills robocopy; the pending `runSync` then resolves with a Cancelled run. `exitWhenDone` closes the app after that. */
export const cancelRun = (exitWhenDone = false) => invoke<boolean>("cancel_run", { exitWhenDone });

export const onRunProgress = (handler: (progress: RunProgress) => void) =>
  listen<RunProgress>("run-progress", (event) => handler(event.payload));

/** The user tried to close the window while a run is going; the window stays open. */
export const onCloseRequested = (handler: () => void) => listen("close-requested", () => handler());
