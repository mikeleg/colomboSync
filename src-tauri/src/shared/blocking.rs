use super::error_messages::{self, Explanation};
use tauri::async_runtime::spawn_blocking;

/// Runs `work` on a blocking thread, so a slow disk never freezes the UI.
pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, Explanation> + Send + 'static,
) -> Result<T, Explanation> {
    spawn_blocking(work)
        .await
        .map_err(|_| error_messages::unexpected_failure())?
}
