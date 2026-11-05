use tauri::async_runtime::spawn_blocking;

/// Runs `work` on a blocking thread, so a slow disk never freezes the UI.
pub async fn blocking<T: Send + 'static, E: From<String> + Send + 'static>(
    work: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> Result<T, E> {
    spawn_blocking(work)
        .await
        .map_err(|e| E::from(e.to_string()))?
}
