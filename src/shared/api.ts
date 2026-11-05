// Calls used by the app shell (App.svelte) and by components two slices share (LogViewer).
import { invoke } from "@tauri-apps/api/core";

/** The app was started as administrator: mapped network drives are then not visible. */
export const isElevated = () => invoke<boolean>("is_elevated");

/** Text of a run's robocopy log; rejects when the file is gone. */
export const readRunLog = (path: string) => invoke<string>("read_run_log", { path });
