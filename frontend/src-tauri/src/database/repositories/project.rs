use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, FromRow, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

pub struct ProjectsRepository;

impl ProjectsRepository {
    pub async fn list(pool: &SqlitePool) -> Result<Vec<Project>, sqlx::Error> {
        sqlx::query_as("SELECT id, name, created_at FROM projects ORDER BY name COLLATE NOCASE")
            .fetch_all(pool)
            .await
    }

    /// Names are unique ignoring case; a duplicate returns a database (unique) error.
    pub async fn create(pool: &SqlitePool, name: &str) -> Result<Project, sqlx::Error> {
        let project = Project {
            id: format!("project-{}", Uuid::new_v4()),
            name: name.trim().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        sqlx::query("INSERT INTO projects (id, name, created_at) VALUES (?, ?, ?)")
            .bind(&project.id)
            .bind(&project.name)
            .bind(&project.created_at)
            .execute(pool)
            .await?;
        Ok(project)
    }

    /// Returns false if the project does not exist.
    pub async fn rename(pool: &SqlitePool, id: &str, name: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("UPDATE projects SET name = ? WHERE id = ?")
            .bind(name.trim())
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Deletes the project and unassigns its meetings (the meetings are kept). Returns false if
    /// the project does not exist. Unassigns explicitly so it does not depend on SQLite's
    /// foreign_keys pragma being on.
    pub async fn delete(pool: &SqlitePool, id: &str) -> Result<bool, sqlx::Error> {
        let mut tx = pool.begin().await?;
        sqlx::query("UPDATE meetings SET project_id = NULL WHERE project_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }

    /// Assigns a meeting to a project, or unassigns it with `None`. Returns false if the
    /// meeting does not exist.
    pub async fn set_meeting_project(
        pool: &SqlitePool,
        meeting_id: &str,
        project_id: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("UPDATE meetings SET project_id = ? WHERE id = ?")
            .bind(project_id)
            .bind(meeting_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool_with_meeting() -> SqlitePool {
        // One connection: each in-memory SQLite connection is a separate database
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at) VALUES ('meeting-1', 'Call', '2026-11-03', '2026-11-03')",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    async fn project_of(pool: &SqlitePool, meeting_id: &str) -> Option<String> {
        sqlx::query_scalar("SELECT project_id FROM meetings WHERE id = ?")
            .bind(meeting_id)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn creates_lists_and_rejects_duplicate_names() {
        let pool = pool_with_meeting().await;
        ProjectsRepository::create(&pool, "  zaroba ")
            .await
            .unwrap();
        ProjectsRepository::create(&pool, "Acme").await.unwrap();
        assert!(ProjectsRepository::create(&pool, "ZAROBA").await.is_err());

        let names: Vec<String> = ProjectsRepository::list(&pool)
            .await
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, vec!["Acme", "zaroba"]);
    }

    #[tokio::test]
    async fn assigns_renames_and_deleting_keeps_the_meeting() {
        let pool = pool_with_meeting().await;
        let project = ProjectsRepository::create(&pool, "Acme").await.unwrap();

        assert!(
            ProjectsRepository::set_meeting_project(&pool, "meeting-1", Some(&project.id))
                .await
                .unwrap()
        );
        assert_eq!(
            project_of(&pool, "meeting-1").await,
            Some(project.id.clone())
        );
        assert!(
            !ProjectsRepository::set_meeting_project(&pool, "missing", Some(&project.id))
                .await
                .unwrap()
        );

        assert!(ProjectsRepository::rename(&pool, &project.id, "Acme Ltd")
            .await
            .unwrap());
        assert_eq!(
            ProjectsRepository::list(&pool).await.unwrap()[0].name,
            "Acme Ltd"
        );

        assert!(ProjectsRepository::delete(&pool, &project.id)
            .await
            .unwrap());
        assert!(ProjectsRepository::list(&pool).await.unwrap().is_empty());
        assert_eq!(project_of(&pool, "meeting-1").await, None);
        let meetings: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM meetings")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(meetings, 1);
    }
}
