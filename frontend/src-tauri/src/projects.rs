// Tauri commands for projects: group meetings by client/project.

use crate::database::repositories::project::{Project, ProjectsRepository};
use crate::state::AppState;

fn validated_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Project name cannot be empty".to_string());
    }
    Ok(name)
}

fn describe(e: sqlx::Error, name: &str) -> String {
    match &e {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            format!("A project named \"{}\" already exists", name)
        }
        _ => e.to_string(),
    }
}

#[tauri::command]
pub async fn api_list_projects(state: tauri::State<'_, AppState>) -> Result<Vec<Project>, String> {
    ProjectsRepository::list(state.db_manager.pool())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn api_create_project(
    state: tauri::State<'_, AppState>,
    name: String,
) -> Result<Project, String> {
    let name = validated_name(&name)?;
    let pool = state.db_manager.pool();
    let project = ProjectsRepository::create(pool, name)
        .await
        .map_err(|e| describe(e, name))?;
    file_meetings_naming(pool, &project.id).await;
    Ok(project)
}

/// Files existing meetings that name the project; a failure here is logged, not returned,
/// since the project change itself succeeded.
async fn file_meetings_naming(pool: &sqlx::SqlitePool, project_id: &str) {
    if let Err(e) = ProjectsRepository::file_meetings_naming(pool, project_id).await {
        log::warn!("Could not file meetings into the project: {}", e);
    }
}

#[tauri::command]
pub async fn api_rename_project(
    state: tauri::State<'_, AppState>,
    project_id: String,
    name: String,
) -> Result<(), String> {
    let name = validated_name(&name)?;
    let pool = state.db_manager.pool();
    match ProjectsRepository::rename(pool, &project_id, name).await {
        Ok(true) => {
            file_meetings_naming(pool, &project_id).await;
            Ok(())
        }
        Ok(false) => Err("Project not found".to_string()),
        Err(e) => Err(describe(e, name)),
    }
}

#[tauri::command]
pub async fn api_delete_project(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<(), String> {
    match ProjectsRepository::delete(state.db_manager.pool(), &project_id).await {
        Ok(true) => Ok(()),
        Ok(false) => Err("Project not found".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn api_set_meeting_project(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    project_id: Option<String>,
) -> Result<(), String> {
    match ProjectsRepository::set_meeting_project(
        state.db_manager.pool(),
        &meeting_id,
        project_id.as_deref(),
    )
    .await
    {
        Ok(true) => Ok(()),
        Ok(false) => Err("Meeting not found".to_string()),
        Err(e) => Err(e.to_string()),
    }
}
