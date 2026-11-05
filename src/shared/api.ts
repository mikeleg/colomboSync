// Calls used by the app shell (App.svelte) rather than by one slice.
import { invoke } from "@tauri-apps/api/core";

/** The app was started as administrator: mapped network drives are then not visible. */
export const isElevated = () => invoke<boolean>("is_elevated");
