// Calls used by the app shell (App.svelte) and by components two slices share (LogViewer).
import { invoke } from "@tauri-apps/api/core";
import type { RunLog } from "./types";

/** The app was started as administrator: mapped network drives are then not visible. */
export const isElevated = () => invoke<boolean>("is_elevated");

/** A run's engine log with its error lines; rejects when the file is gone. */
export const readRunLog = (path: string) => invoke<RunLog>("read_run_log", { path });
