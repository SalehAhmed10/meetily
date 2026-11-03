use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, PartialEq)]
pub struct TimelineEntry {
    pub id: String,
    pub title: String,
    /// "meeting" or "email"
    pub kind: String,
    pub stage: Option<String>,
    pub date: String,
    pub has_summary: bool,
}

pub struct TimelineRepository;

impl TimelineRepository {
    /// The project's meetings and emails, oldest first.
    pub async fn project_timeline(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Vec<TimelineEntry>, sqlx::Error> {
        sqlx::query_as(
            "SELECT m.id, m.title, m.kind, m.stage, CAST(m.created_at AS TEXT) AS date,
                    EXISTS (SELECT 1 FROM summary_processes s
                            WHERE s.meeting_id = m.id AND LOWER(s.status) = 'completed') AS has_summary
             FROM meetings m WHERE m.project_id = ? ORDER BY m.created_at ASC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    /// Sets or clears (None or blank) a meeting's stage. Returns false if the meeting is missing.
    pub async fn set_stage(
        pool: &SqlitePool,
        meeting_id: &str,
        stage: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let stage = stage.map(str::trim).filter(|s| !s.is_empty());
        let result = sqlx::query("UPDATE meetings SET stage = ? WHERE id = ?")
            .bind(stage)
            .bind(meeting_id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Saves an email into the project, dated when it was sent, one transcript line per
    /// paragraph. Returns the new meeting id.
    pub async fn add_email(
        pool: &SqlitePool,
        project_id: &str,
        title: &str,
        body: &str,
        sent_at: DateTime<Utc>,
        stage: Option<&str>,
    ) -> Result<String, sqlx::Error> {
        let meeting_id = format!("meeting-{}", Uuid::new_v4());
        let stage = stage.map(str::trim).filter(|s| !s.is_empty());
        let mut tx = pool.begin().await?;
        sqlx::query(
            "INSERT INTO meetings (id, title, created_at, updated_at, project_id, kind, stage)
             VALUES (?, ?, ?, ?, ?, 'email', ?)",
        )
        .bind(&meeting_id)
        .bind(title.trim())
        .bind(sent_at)
        .bind(Utc::now())
        .bind(project_id)
        .bind(stage)
        .execute(&mut *tx)
        .await?;
        for paragraph in paragraphs(body) {
            sqlx::query(
                "INSERT INTO transcripts (id, meeting_id, transcript, timestamp) VALUES (?, ?, ?, '')",
            )
            .bind(format!("transcript-{}", Uuid::new_v4()))
            .bind(&meeting_id)
            .bind(paragraph)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(meeting_id)
    }
}

/// Blank-line separated paragraphs, trimmed, empty ones dropped
fn paragraphs(body: &str) -> Vec<String> {
    body.replace("\r\n", "\n")
        .split("\n\n")
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::repositories::project::ProjectsRepository;

    #[test]
    fn splits_paragraphs() {
        assert_eq!(
            paragraphs("Hi Saleh,\r\n\r\nCongrats!\nRound 2 next.\n\n\n  "),
            vec!["Hi Saleh,", "Congrats!\nRound 2 next."]
        );
    }

    #[tokio::test]
    async fn emails_and_stages_on_the_real_schema() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let project = ProjectsRepository::create(&pool, "Remote Haven")
            .await
            .unwrap();
        sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at, project_id) VALUES ('m1', 'Interview', '2026-11-03T10:00:00Z', '2026-11-03T10:00:00Z', ?)")
            .bind(&project.id)
            .execute(&pool)
            .await
            .unwrap();

        let sent = DateTime::parse_from_rfc3339("2026-11-05T09:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let email = TimelineRepository::add_email(
            &pool,
            &project.id,
            " Round 2 invite ",
            "Hi,\n\nYou're through.",
            sent,
            Some("Round 1"),
        )
        .await
        .unwrap();
        assert!(
            TimelineRepository::set_stage(&pool, "m1", Some("  Round 1 "))
                .await
                .unwrap()
        );
        assert!(!TimelineRepository::set_stage(&pool, "missing", Some("x"))
            .await
            .unwrap());

        let timeline = TimelineRepository::project_timeline(&pool, &project.id)
            .await
            .unwrap();
        let view: Vec<_> = timeline
            .iter()
            .map(|e| (e.title.as_str(), e.kind.as_str(), e.stage.as_deref()))
            .collect();
        assert_eq!(
            view,
            vec![
                ("Interview", "meeting", Some("Round 1")),
                ("Round 2 invite", "email", Some("Round 1"))
            ]
        );
        assert!(timeline[1].date.starts_with("2026-11-05"));

        let lines: Vec<String> =
            sqlx::query_scalar("SELECT transcript FROM transcripts WHERE meeting_id = ?")
                .bind(&email)
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(lines.len(), 2);

        TimelineRepository::set_stage(&pool, "m1", Some("   "))
            .await
            .unwrap(); // blank clears
        assert_eq!(
            TimelineRepository::project_timeline(&pool, &project.id)
                .await
                .unwrap()[0]
                .stage,
            None
        );
    }
}
