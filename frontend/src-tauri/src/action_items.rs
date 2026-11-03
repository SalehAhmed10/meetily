// Open action items per project: read from each meeting's summary, ticked off in the app.

use crate::database::repositories::action_item::{ActionItemsRepository, ProjectSummary};
use crate::state::AppState;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActionItem {
    pub owner: Option<String>,
    pub task: String,
    pub due: Option<String>,
    /// Transcript time the item came from, as the summary wrote it ("02:52")
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProjectActionItem {
    pub meeting_id: String,
    pub meeting_title: String,
    pub meeting_date: String,
    pub key: String,
    pub done: bool,
    #[serde(flatten)]
    pub item: ActionItem,
}

/// What a ticked-off item is remembered by: owner and task, lowercased, spaces collapsed
fn item_key(item: &ActionItem) -> String {
    format!("{} {}", item.owner.as_deref().unwrap_or(""), item.task)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Bold lines rank below every `#` level, so "**Saleh**" inside "## Action Items" is a subheading
const BOLD_LEVEL: usize = 7;

/// (level, text) of a heading line ("## Action Items", "**Client Action Items**:"), or None
fn heading(line: &str) -> Option<(usize, &str)> {
    let line = line.trim();
    if line.starts_with('#') {
        let level = line.chars().take_while(|&c| c == '#').count();
        return Some((level, line[level..].trim()));
    }
    let inner = line
        .trim_end_matches(':')
        .strip_prefix("**")?
        .strip_suffix("**")?;
    (!inner.is_empty() && !inner.contains("**"))
        .then(|| (BOLD_LEVEL, inner.trim().trim_end_matches(':')))
}

/// "**Saleh:** Send scope" -> (Saleh, Send scope); a short bold label before the words
fn owner_prefix(text: &str) -> Option<(String, &str)> {
    let rest = text.strip_prefix("**")?;
    let end = rest.find("**")?;
    let label = rest[..end].trim().trim_end_matches(':').trim();
    let words = rest[end + 2..].trim_start().trim_start_matches(':').trim();
    (!label.is_empty() && !words.is_empty() && label.split_whitespace().count() <= 4)
        .then(|| (label.to_string(), words))
}

fn clean(cell: &str) -> Option<String> {
    let text = cell.replace("**", "").trim().to_string();
    let empty = matches!(
        text.to_lowercase().as_str(),
        "" | "-"
            | "n/a"
            | "na"
            | "none"
            | "tbd"
            | "tbc"
            | "pending"
            | "not specified"
            | "unspecified"
    );
    (!empty).then_some(text)
}

/// Table cells; `\|` stays inside its cell
fn cells(row: &str) -> Vec<String> {
    let row = row.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    let row = row.strip_suffix('|').unwrap_or(row);
    row.replace("\\|", "\u{1}")
        .split('|')
        .map(|c| c.trim().replace('\u{1}', "|"))
        .collect()
}

struct Section {
    level: usize,
    owner: Option<String>,
}

/// Action items from a summary: every section whose heading says "action item", written
/// either as a table (Owner | Task | Due | ... | time) or as a list. A heading like
/// "Client Action Items", a deeper subheading inside the section, or a bold "**Saleh:**"
/// before an item sets the owner. The section ends at the next heading of its level or above.
pub fn parse_action_items(markdown: &str) -> Vec<ActionItem> {
    let mut items: Vec<ActionItem> = Vec::new();
    let mut section: Option<Section> = None;
    let mut columns: Option<Vec<String>> = None; // table header of the current section
    let mut last_bullet: Option<usize> = None; // index in items, for nested bullets

    for line in markdown.lines() {
        let trimmed = line.trim();
        if let Some((level, title)) = heading(trimmed) {
            let lower = title.to_lowercase();
            columns = None;
            last_bullet = None;
            if let Some(at) = lower.find("action item") {
                let owner = clean(title[..at].trim().trim_end_matches(['-', ':']));
                section = Some(Section { level, owner });
            } else if let Some(open) = section.as_mut().filter(|s| level > s.level) {
                open.owner = clean(title);
            } else {
                section = None;
            }
            continue;
        }
        let Some(Section { owner, .. }) = &section else {
            continue;
        };

        if trimmed.starts_with('|') {
            last_bullet = None;
            let row = cells(trimmed);
            if row
                .iter()
                .all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':')))
            {
                continue; // | --- | :--- |
            }
            let Some(header) = &columns else {
                columns = Some(
                    row.iter()
                        .map(|c| c.replace("**", "").to_lowercase())
                        .collect(),
                );
                continue;
            };
            let find = |words: &[&str]| {
                header
                    .iter()
                    .position(|h| words.iter().any(|w| h.contains(w)))
            };
            let owner_col = find(&["owner", "assignee", "who"]);
            let task_col = find(&["task", "action", "item"])
                .or_else(|| (0..header.len()).find(|&i| Some(i) != owner_col));
            let due_col = find(&["due", "deadline", "when"]);
            let time_col = find(&["time"]);
            // A stray `|` inside a quoted cell adds cells; the last column is still the last
            let cell = |col: Option<usize>| {
                col.and_then(
                    |i| match row.len() > header.len() && i + 1 == header.len() {
                        true => row.last(),
                        false => row.get(i),
                    },
                )
                .and_then(|c| clean(c))
            };
            if let Some(task) = cell(task_col) {
                items.push(ActionItem {
                    owner: cell(owner_col).or_else(|| owner.clone()),
                    task,
                    due: cell(due_col),
                    timestamp: cell(time_col),
                });
            }
            continue;
        }

        let bullet = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
            .or_else(|| {
                let digits = trimmed.find(|c: char| !c.is_ascii_digit())?;
                let rest = &trimmed[digits..];
                (digits > 0).then(|| rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")))?
            });
        let Some(text) = bullet else {
            continue;
        };
        let text = ["[ ] ", "[x] ", "[X] "]
            .iter()
            .find_map(|box_| text.strip_prefix(box_))
            .unwrap_or(text);
        let nested = line.len() - line.trim_start().len() >= 2;
        match (nested, last_bullet) {
            // "  - include the timeline" belongs to the item above it
            (true, Some(i)) => {
                if let Some(more) = clean(text) {
                    items[i].task = format!("{}; {}", items[i].task, more);
                }
            }
            _ => {
                let (owner, text) = match owner_prefix(text) {
                    Some((label, words)) => (Some(label), words),
                    None => (owner.clone(), text),
                };
                if let Some(task) = clean(text) {
                    items.push(ActionItem {
                        owner,
                        task,
                        due: None,
                        timestamp: None,
                    });
                    last_bullet = Some(items.len() - 1);
                }
            }
        }
    }
    items
}

#[tauri::command]
pub async fn api_get_project_action_items(
    state: tauri::State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ProjectActionItem>, String> {
    let pool = state.db_manager.pool();
    let summaries = ActionItemsRepository::project_summaries(pool, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    let done: HashSet<(String, String)> = ActionItemsRepository::done_keys(pool, &project_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    Ok(collect_items(summaries, &done))
}

fn collect_items(
    summaries: Vec<ProjectSummary>,
    done: &HashSet<(String, String)>,
) -> Vec<ProjectActionItem> {
    summaries
        .into_iter()
        .flat_map(
            |(meeting_id, meeting_title, meeting_date, markdown, english)| {
                let mut found = parse_action_items(&markdown);
                // A translated summary's headings ("Aktionspunkte") are not found; its English
                // original is kept next to it
                if found.is_empty() {
                    found = english
                        .as_deref()
                        .map(parse_action_items)
                        .unwrap_or_default();
                }
                let mut seen = HashSet::new();
                found
                    .into_iter()
                    // The same owner and task twice in one summary is one item
                    .filter(|item| seen.insert(item_key(item)))
                    .map(|item| {
                        let key = item_key(&item);
                        ProjectActionItem {
                            done: done.contains(&(meeting_id.clone(), key.clone())),
                            meeting_id: meeting_id.clone(),
                            meeting_title: meeting_title.clone(),
                            meeting_date: meeting_date.clone(),
                            key,
                            item,
                        }
                    })
                    .collect::<Vec<_>>()
            },
        )
        .collect()
}

#[tauri::command]
pub async fn api_set_action_item_done(
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    key: String,
    done: bool,
) -> Result<(), String> {
    ActionItemsRepository::set_done(state.db_manager.pool(), &meeting_id, &key, done)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(owner: Option<&str>, task: &str, due: Option<&str>, time: Option<&str>) -> ActionItem {
        ActionItem {
            owner: owner.map(String::from),
            task: task.to_string(),
            due: due.map(String::from),
            timestamp: time.map(String::from),
        }
    }

    #[test]
    fn reads_the_action_items_table_the_summaries_write() {
        let md = "**Summary**\nScope agreed.\n\n**Action Items**\n\n\
            | **Owner** | Task | Due | Reference Transcript Segment | Segment Time stamp |\n\
            | :--- | --- | --- | --- | --- |\n\
            | Saleh Ahmed | Send **written** scope | Friday | \"[02:52] I'll send it\" | 02:52 |\n\
            | Priya | Share brand assets | Pending | \"...\" | 03:10 |\n\
            | | | | | |\n\n\
            **Discussion Highlights**\n- Not an action item\n";
        assert_eq!(
            parse_action_items(md),
            vec![
                item(
                    Some("Saleh Ahmed"),
                    "Send written scope",
                    Some("Friday"),
                    Some("02:52")
                ),
                item(Some("Priya"), "Share brand assets", None, Some("03:10")),
            ]
        );
    }

    #[test]
    fn reads_lists_and_takes_the_owner_from_the_heading() {
        let md = "## Client Action Items\n- [ ] Send the logo files\n* Confirm budget\n\
            ## Developer Action Items\n1. Draft the scope\n2. None\n\
            ## Next Steps\n- Not collected\n\
            **Action Items:**\n- Book the demo\n";
        assert_eq!(
            parse_action_items(md),
            vec![
                item(Some("Client"), "Send the logo files", None, None),
                item(Some("Client"), "Confirm budget", None, None),
                item(Some("Developer"), "Draft the scope", None, None),
                item(None, "Book the demo", None, None),
            ]
        );
        assert!(parse_action_items("**Summary**\n- a point\n").is_empty());
    }

    #[test]
    fn marks_done_items_and_drops_duplicates() {
        let md = "## Action Items\n- Send scope\n- send   SCOPE\n- Book demo\n\
            ### Client\n- Send scope\n"
            .to_string();
        let summaries = vec![(
            "m1".into(),
            "Acme call".into(),
            "2026-11-03".into(),
            md,
            None,
        )];
        let done = HashSet::from([("m1".to_string(), "book demo".to_string())]);
        let items = collect_items(summaries, &done);
        let view: Vec<_> = items
            .iter()
            .map(|i| (i.item.owner.as_deref(), i.item.task.as_str(), i.done))
            .collect();
        // Same task under another owner is its own item
        assert_eq!(
            view,
            vec![
                (None, "Send scope", false),
                (None, "Book demo", true),
                (Some("Client"), "Send scope", false)
            ]
        );
    }

    #[test]
    fn translated_summary_falls_back_to_its_english_original() {
        let german = "**Aktionspunkte**\n- Umfang senden\n".to_string();
        let english = Some("**Action Items**\n- Send scope\n".to_string());
        let summaries = vec![(
            "m1".into(),
            "Acme".into(),
            "2026-11-03".into(),
            german,
            english,
        )];
        let items = collect_items(summaries, &HashSet::new());
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item.task, "Send scope");
    }

    #[test]
    fn sections_survive_subheadings_and_other_markup() {
        // A deeper subheading or bold line inside the section names the owner
        let md = "## Action Items\n### Saleh\n- Send scope\n**Priya**\n1) Share assets\n\
            - **Omar:** Book the demo\n  - include the agenda\n## Notes\n- not collected\n\
            **Action Items**:\n- Colon outside the bold\n";
        assert_eq!(
            parse_action_items(md),
            vec![
                item(Some("Saleh"), "Send scope", None, None),
                item(Some("Priya"), "Share assets", None, None),
                item(
                    Some("Omar"),
                    "Book the demo; include the agenda",
                    None,
                    None
                ),
                item(None, "Colon outside the bold", None, None),
            ]
        );
        // A quoted `|` in the transcript cell does not move the time column
        let table = "**Action Items**\n| Owner | Task | Due | Reference | Time |\n| --- | --- | --- | --- | --- |\n\
            | Saleh | Send scope | Fri | \"[02:52] A | B\" | 02:52 |\n| Priya | Pay \\| invoice | - | x | 03:10 |\n";
        assert_eq!(
            parse_action_items(table),
            vec![
                item(Some("Saleh"), "Send scope", Some("Fri"), Some("02:52")),
                item(Some("Priya"), "Pay | invoice", None, Some("03:10")),
            ]
        );
    }

    #[tokio::test]
    async fn ticks_survive_on_the_real_schema() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let project =
            crate::database::repositories::project::ProjectsRepository::create(&pool, "Acme")
                .await
                .unwrap();
        sqlx::query("INSERT INTO meetings (id, title, created_at, updated_at, project_id) VALUES ('m1', 'Acme call', '2026-11-03', '2026-11-03', ?)")
            .bind(&project.id)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO summary_processes (meeting_id, status, created_at, updated_at, result) VALUES ('m1', 'completed', '2026-11-03', '2026-11-03', ?)")
            .bind(r#"{"markdown":"**Action Items**\n- Send scope\n"}"#)
            .execute(&pool)
            .await
            .unwrap();

        let summaries = ActionItemsRepository::project_summaries(&pool, &project.id)
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].1, "Acme call");

        ActionItemsRepository::set_done(&pool, "m1", "send scope", true)
            .await
            .unwrap();
        ActionItemsRepository::set_done(&pool, "m1", "send scope", true)
            .await
            .unwrap(); // twice is fine
        let done = ActionItemsRepository::done_keys(&pool, &project.id)
            .await
            .unwrap();
        assert_eq!(done, vec![("m1".to_string(), "send scope".to_string())]);
        ActionItemsRepository::set_done(&pool, "m1", "send scope", false)
            .await
            .unwrap();
        assert!(ActionItemsRepository::done_keys(&pool, &project.id)
            .await
            .unwrap()
            .is_empty());
    }
}
