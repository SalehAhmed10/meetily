use sqlx::SqlitePool;

/// A completed summary of a meeting in a project: (meeting id, title, created at, markdown,
/// English markdown when the summary was translated)
pub type ProjectSummary = (String, String, String, String, Option<String>);

pub struct ActionItemsRepository;

impl ActionItemsRepository {
    /// Completed summaries of the project's meetings, newest meeting first.
    pub async fn project_summaries(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Vec<ProjectSummary>, sqlx::Error> {
        sqlx::query_as(
            "SELECT m.id, m.title, CAST(m.created_at AS TEXT), json_extract(s.result, '$.markdown'),
                    json_extract(s.result, '$.english_cache.markdown')
             FROM meetings m JOIN summary_processes s ON s.meeting_id = m.id
             WHERE m.project_id = ? AND LOWER(s.status) = 'completed' AND json_valid(s.result)
               AND json_extract(s.result, '$.markdown') IS NOT NULL
             ORDER BY m.created_at DESC",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    /// (meeting id, item key) of every ticked-off item in the project's meetings.
    pub async fn done_keys(
        pool: &SqlitePool,
        project_id: &str,
    ) -> Result<Vec<(String, String)>, sqlx::Error> {
        sqlx::query_as(
            "SELECT a.meeting_id, a.item_key FROM action_item_status a
             JOIN meetings m ON m.id = a.meeting_id WHERE m.project_id = ?",
        )
        .bind(project_id)
        .fetch_all(pool)
        .await
    }

    pub async fn set_done(
        pool: &SqlitePool,
        meeting_id: &str,
        item_key: &str,
        done: bool,
    ) -> Result<(), sqlx::Error> {
        let query = if done {
            sqlx::query(
                "INSERT OR REPLACE INTO action_item_status (meeting_id, item_key, done_at)
                 VALUES (?, ?, ?)",
            )
            .bind(meeting_id)
            .bind(item_key)
            .bind(chrono::Utc::now().to_rfc3339())
        } else {
            sqlx::query("DELETE FROM action_item_status WHERE meeting_id = ? AND item_key = ?")
                .bind(meeting_id)
                .bind(item_key)
        };
        query.execute(pool).await.map(|_| ())
    }
}
