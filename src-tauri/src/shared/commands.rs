use super::elevation;

/// The app runs as administrator: mapped network drives are then not visible.
#[tauri::command]
pub fn is_elevated() -> bool {
    elevation::is_elevated()
}
