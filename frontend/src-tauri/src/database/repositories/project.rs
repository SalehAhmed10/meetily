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

    /// Files a meeting that has no project into the project whose name appears in its title
    /// ("Zaroba weekly sync" -> Zaroba). Never moves a meeting that already has a project.
    /// Returns the project id it was filed into.
    pub async fn auto_assign(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Option<String>, sqlx::Error> {
        let title: Option<String> =
            sqlx::query_scalar("SELECT title FROM meetings WHERE id = ? AND project_id IS NULL")
                .bind(meeting_id)
                .fetch_optional(pool)
                .await?;
        let Some(title) = title else {
            return Ok(None);
        };
        let projects = Self::list(pool).await?;
        let Some(project) = project_named_in(&title, &projects) else {
            return Ok(None);
        };
        // A project set by hand between the SELECT and here is kept, and nothing was filed
        let filed =
            sqlx::query("UPDATE meetings SET project_id = ? WHERE id = ? AND project_id IS NULL")
                .bind(&project.id)
                .bind(meeting_id)
                .execute(pool)
                .await?
                .rows_affected()
                > 0;
        Ok(filed.then(|| project.id.clone()))
    }
}

/// The project whose name appears in the title as a whole word, ignoring case. The longest
/// name wins ("Acme Mobile" beats "Acme"); between names of equal length, the one earlier in
/// the title.
fn project_named_in<'a>(title: &str, projects: &'a [Project]) -> Option<&'a Project> {
    let title = title.to_lowercase();
    let is_word = |c: char| c.is_alphanumeric();
    projects
        .iter()
        .filter_map(|p| {
            let name = p.name.trim().to_lowercase();
            if name.is_empty() {
                return None;
            }
            // Every start, not match_indices: that skips overlapping matches
            let at = title.char_indices().map(|(i, _)| i).find(|&i| {
                title[i..].starts_with(&name)
                    && !title[..i].chars().next_back().is_some_and(is_word)
                    && !title[i + name.len()..].chars().next().is_some_and(is_word)
            })?;
            Some((name.chars().count(), std::cmp::Reverse(at), p))
        })
        .max_by_key(|&(len, at, _)| (len, at))
        .map(|(_, _, p)| p)
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

    fn project(name: &str) -> Project {
        Project {
            id: name.to_string(),
            name: name.to_string(),
            created_at: String::new(),
        }
    }

    #[test]
    fn finds_the_longest_project_named_as_a_whole_word() {
        let projects = [
            project("Acme"),
            project("Acme Mobile"),
            project("AI"),
            project("C++"),
        ];
        let named = |title: &str| project_named_in(title, &projects).map(|p| p.name.as_str());
        assert_eq!(named("acme weekly sync"), Some("Acme"));
        assert_eq!(named("Kickoff - ACME MOBILE app"), Some("Acme Mobile"));
        assert_eq!(named("Email triage"), None); // "AI" inside a word
        assert_eq!(named("Acmeville call"), None);
        assert_eq!(named("C++ review"), Some("C++"));
        assert_eq!(named("Meeting 2026-11-03"), None);

        // Sorted as list() returns them; the old max_by_key picked the last tie (Beta)
        let projects = [project("Acme"), project("ab ab"), project("Beta")];
        let named = |title: &str| project_named_in(title, &projects).map(|p| p.name.as_str());
        assert_eq!(named("Acme Beta sync"), Some("Acme")); // same length: earlier in the title
        assert_eq!(named("xab ab ab"), Some("ab ab")); // overlapping match after a failed one
    }

    #[tokio::test]
    async fn auto_assign_files_only_meetings_without_a_project() {
        let pool = pool_with_meeting().await;
        sqlx::query("UPDATE meetings SET title = 'Zaroba weekly sync' WHERE id = 'meeting-1'")
            .execute(&pool)
            .await
            .unwrap();
        assert_eq!(
            ProjectsRepository::auto_assign(&pool, "meeting-1")
                .await
                .unwrap(),
            None
        );

        let zaroba = ProjectsRepository::create(&pool, "Zaroba").await.unwrap();
        assert_eq!(
            ProjectsRepository::auto_assign(&pool, "meeting-1")
                .await
                .unwrap(),
            Some(zaroba.id.clone())
        );
        assert_eq!(
            project_of(&pool, "meeting-1").await,
            Some(zaroba.id.clone())
        );

        // A project chosen by hand is never overridden
        let other = ProjectsRepository::create(&pool, "Weekly").await.unwrap();
        ProjectsRepository::set_meeting_project(&pool, "meeting-1", Some(&other.id))
            .await
            .unwrap();
        assert_eq!(
            ProjectsRepository::auto_assign(&pool, "meeting-1")
                .await
                .unwrap(),
            None
        );
        assert_eq!(project_of(&pool, "meeting-1").await, Some(other.id));
        assert_eq!(
            ProjectsRepository::auto_assign(&pool, "missing")
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn renaming_a_meeting_files_it() {
        use crate::database::repositories::meeting::MeetingsRepository;
        let pool = pool_with_meeting().await; // one connection: also proves no deadlock
        let acme = ProjectsRepository::create(&pool, "Acme").await.unwrap();
        assert!(
            MeetingsRepository::update_meeting_title(&pool, "meeting-1", "Acme kickoff")
                .await
                .unwrap()
        );
        assert_eq!(project_of(&pool, "meeting-1").await, Some(acme.id.clone()));

        ProjectsRepository::set_meeting_project(&pool, "meeting-1", None)
            .await
            .unwrap();
        assert!(
            MeetingsRepository::update_meeting_name(&pool, "meeting-1", "Acme scope call")
                .await
                .unwrap()
        );
        assert_eq!(project_of(&pool, "meeting-1").await, Some(acme.id));
    }
}
