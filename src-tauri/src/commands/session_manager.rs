#![allow(non_snake_case)]

use crate::session_manager;
use tokio::task::spawn_blocking;

pub async fn list_sessions_internal() -> Result<Vec<session_manager::SessionMeta>, String> {
    let sessions = spawn_blocking(session_manager::scan_sessions)
        .await
        .map_err(|e| format!("Failed to scan sessions: {e}"))?;
    Ok(sessions)
}

pub async fn get_session_messages_internal(
    providerId: String,
    sourcePath: String,
) -> Result<Vec<session_manager::SessionMessage>, String> {
    let provider_id = providerId.clone();
    let source_path = sourcePath.clone();
    spawn_blocking(move || session_manager::load_messages(&provider_id, &source_path))
    .await
    .map_err(|e| format!("Failed to load session messages: {e}"))?
}

pub async fn delete_session_internal(
    providerId: String,
    sessionId: String,
    sourcePath: String,
) -> Result<bool, String> {
    let provider_id = providerId.clone();
    let session_id = sessionId.clone();
    let source_path = sourcePath.clone();

    spawn_blocking(move || {
        session_manager::delete_session(&provider_id, &session_id, &source_path)
    })
    .await
    .map_err(|e| format!("Failed to delete session: {e}"))?
}

pub async fn delete_sessions_internal(
    items: Vec<session_manager::DeleteSessionRequest>,
) -> Result<Vec<session_manager::DeleteSessionOutcome>, String> {
    spawn_blocking(move || session_manager::delete_sessions(&items))
        .await
        .map_err(|e| format!("Failed to delete sessions: {e}"))
}
