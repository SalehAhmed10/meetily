// A project's timeline: its meetings and emails in date order, grouped by stage.

use crate::database::repositories::timeline::{TimelineEntry, TimelineRepository};
use crate::state::AppState;
use chrono::{DateTime, Utc};

#[tauri::command]
pub async fn api_get_project_timeline(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<TimelineEntry>, String> {
    TimelineRepository::project_timeline(state.db_manager.pool(), &project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn api_set_meeting_stage(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    stage: Option<String>,
) -> Result<(), String> {
    match TimelineRepository::set_stage(state.db_manager.pool(), &meeting_id, stage.as_deref())
        .await
    {
        Ok(true) => Ok(()),
        Ok(false) => Err("Meeting not found".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// `sent_at` is an RFC 3339 time (the page sends `Date.toISOString()`).
#[tauri::command]
pub async fn api_add_project_email(
    state: tauri::State<'_, AppState>,
    project_id: String,
    title: String,
    body: String,
    sent_at: String,
    stage: Option<String>,
) -> Result<String, String> {
    if title.trim().is_empty() || body.trim().is_empty() {
        return Err("An email needs a subject and some text".to_string());
    }
    let sent_at = DateTime::parse_from_rfc3339(&sent_at)
        .map_err(|e| format!("Bad date: {e}"))?
        .with_timezone(&Utc);
    TimelineRepository::add_email(
        state.db_manager.pool(),
        &project_id,
        &title,
        &body,
        sent_at,
        stage.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}
