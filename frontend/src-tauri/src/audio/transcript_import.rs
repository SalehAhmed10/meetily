// Transcript import - create a meeting from an existing transcript file (.txt/.md/.srt/.vtt/.docx)
// so it can be summarized without any audio or transcription step.

use crate::api::TranscriptSegment;
use crate::state::AppState;
use anyhow::{anyhow, Result};
use log::{info, warn};
use std::io::Read;
use std::path::Path;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use super::audio_processing::create_meeting_folder;
use super::common::write_transcripts_json;
use super::import::{create_meeting_with_transcripts, write_import_metadata, ImportResult};
use super::recording_preferences::get_default_recordings_folder;

const TRANSCRIPT_EXTENSIONS: &[&str] = &["txt", "md", "srt", "vtt", "docx"];

/// Parse transcript text into (text, start_seconds, end_seconds) entries.
/// Subtitle formats keep their cue times; plain text becomes one entry per non-empty line.
fn parse_transcript(content: &str, extension: &str) -> Vec<(String, Option<f64>, Option<f64>)> {
    let content = content.trim_start_matches('\u{feff}');
    if !matches!(extension, "srt" | "vtt") {
        return parse_plain_text(content);
    }

    // Cues are separated by blank lines; some exporters write whitespace on those lines
    let mut blocks: Vec<Vec<&str>> = vec![Vec::new()];
    for line in content.lines().map(str::trim) {
        if line.is_empty() {
            blocks.push(Vec::new());
        } else if let Some(block) = blocks.last_mut() {
            block.push(line);
        }
    }
    blocks
        .into_iter()
        .filter_map(|block| {
            let mut lines = block.into_iter();
            // Skip the cue number / cue id / header lines until the timing line
            let timing = lines.by_ref().find(|l| l.contains("-->"))?;
            let mut times = timing
                .split("-->")
                .map(|t| parse_timestamp(t.split_whitespace().next().unwrap_or("")));
            let start = times.next().flatten();
            let end = times.next().flatten();
            let text = lines.map(strip_cue_tags).collect::<Vec<_>>().join(" ");
            let text = text.trim().to_string();
            (!text.is_empty()).then_some((text, start, end))
        })
        .collect()
}

/// One entry per non-empty line. When most lines start with a timestamp ("00:05 Alex: hi",
/// "[1:02:03] ...", as in Google Meet exports), it becomes the line's start time and each line
/// ends where the next timestamped line starts. "Most" is counted from the first timestamped
/// line, since exports put a header before it, and needs at least two such lines, so ordinary
/// notes that happen to begin with a time ("10:30 call moved") stay plain text.
fn parse_plain_text(content: &str) -> Vec<(String, Option<f64>, Option<f64>)> {
    let lines: Vec<&str> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let stamped: Vec<Option<(f64, &str)>> =
        lines.iter().map(|l| split_leading_timestamp(l)).collect();
    let first_stamped = stamped
        .iter()
        .position(Option::is_some)
        .unwrap_or(lines.len());
    let stamped_count = stamped.iter().filter(|s| s.is_some()).count();
    if stamped_count < 2 || stamped_count * 2 <= lines.len() - first_stamped {
        return lines
            .into_iter()
            .map(|l| (l.to_string(), None, None))
            .collect();
    }

    let mut entries: Vec<(String, Option<f64>, Option<f64>)> = Vec::new();
    for (line, stamp) in lines.iter().zip(&stamped) {
        match (stamp, entries.last_mut()) {
            (Some((start, rest)), _) => entries.push((rest.to_string(), Some(*start), None)),
            // An untimed line inside the transcript (e.g. a list under a speaker's turn)
            // continues the previous line; kept separate it would sort to the top (no time)
            (None, Some(previous)) if previous.1.is_some() => {
                previous.0.push(' ');
                previous.0.push_str(line);
            }
            (None, _) => entries.push((line.to_string(), None, None)),
        }
    }
    let mut next_start: Option<f64> = None;
    for entry in entries.iter_mut().rev() {
        if let Some(start) = entry.1 {
            entry.2 = Some(next_start.unwrap_or(start).max(start));
            next_start = Some(start);
        }
    }
    entries
}

/// "00:05 Alex: hi" / "[00:05] Alex: hi" -> (5.0, "Alex: hi"); None if the line has no leading time
fn split_leading_timestamp(line: &str) -> Option<(f64, &str)> {
    let (token, rest) = line.split_once(char::is_whitespace)?;
    let token = token
        .trim_start_matches(['[', '('])
        .trim_end_matches([']', ')']);
    if !token.contains(':') {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    Some((parse_timestamp(token)?, rest))
}

/// "HH:MM:SS,mmm", "HH:MM:SS.mmm" or "MM:SS.mmm" -> seconds
fn parse_timestamp(ts: &str) -> Option<f64> {
    let parts: Vec<f64> = ts
        .trim()
        .replace(',', ".")
        .split(':')
        .map(|p| p.parse::<f64>().ok())
        .collect::<Option<_>>()?;
    match parts.as_slice() {
        [h, m, s] => Some(h * 3600.0 + m * 60.0 + s),
        [m, s] => Some(m * 60.0 + s),
        _ => None,
    }
}

/// Turn VTT voice tags into "Speaker: " prefixes and drop all other markup tags.
fn strip_cue_tags(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let Some(close) = rest[open..].find('>') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let tag = &rest[open + 1..open + close];
        if let Some(speaker) = tag.strip_prefix("v ") {
            // "<v.loud Name>" / "<v Name>" -> "Name: "
            out.push_str(speaker.trim());
            out.push_str(": ");
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

fn to_segments(entries: Vec<(String, Option<f64>, Option<f64>)>) -> Vec<TranscriptSegment> {
    let now = chrono::Utc::now().to_rfc3339();
    entries
        .into_iter()
        .map(|(text, start, end)| TranscriptSegment {
            id: format!("transcript-{}", Uuid::new_v4()),
            text,
            // Copy and the summary prompt fall back to this when there is no audio time, so a
            // line without a time gets no wall-clock text either
            timestamp: if start.is_some() {
                now.clone()
            } else {
                String::new()
            },
            audio_start_time: start,
            audio_end_time: end,
            duration: start.zip(end).map(|(s, e)| e - s),
        })
        .collect()
}

/// Text of a Word document, one line per paragraph (Teams and Google Docs transcript downloads).
fn docx_text(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path)
        .map_err(|e| anyhow!("Could not open {}: {}", path.display(), e))?;
    let mut xml = String::new();
    zip::ZipArchive::new(file)
        .and_then(|mut archive| {
            archive
                .by_name("word/document.xml")?
                .read_to_string(&mut xml)
                .map_err(Into::into)
        })
        .map_err(|e| anyhow!("{} is not a readable .docx file: {}", path.display(), e))?;
    Ok(docx_xml_to_text(&xml))
}

/// Paragraphs of word/document.xml as lines: text runs joined, tabs as spaces, breaks kept.
fn docx_xml_to_text(xml: &str) -> String {
    // \b keeps <w:p> apart from <w:pPr>, and <w:t> apart from <w:tab>/<w:tbl>
    let paragraph = regex::Regex::new(r"(?s)<w:p\b[^>]*?(?:/>|>(.*?)</w:p>)").unwrap();
    let piece = regex::Regex::new(r"<w:t\b[^>]*>([^<]*)</w:t>|<w:(tab|br|cr)\b[^>]*/>").unwrap();
    paragraph
        .captures_iter(xml)
        .map(|p| {
            let body = p.get(1).map_or("", |m| m.as_str());
            piece
                .captures_iter(body)
                .map(|c| match c.get(2).map(|m| m.as_str()) {
                    Some("tab") => " ".to_string(),
                    Some(_) => "\n".to_string(),
                    None => unescape_xml(&c[1]),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Shapes .docx transcript text for parse_transcript, returning it with the format to parse as.
/// Cue timing lines ("0:0:1.5 --> 0:0:4.0", older Teams exports) get the blank line before them
/// that VTT cues need; "Name   0:03" lines followed by the words (newer Teams exports) become
/// "0:03 Name: words" so the plain-text parser keeps the time and the speaker.
// ponytail: layouts from memory of Teams/Meet exports, not real files; tune with a real sample
fn docx_transcript(text: &str) -> (String, &'static str) {
    let is_cue_timing = |l: &str| {
        l.split_once("-->").is_some_and(|(a, b)| {
            parse_timestamp(a.trim()).is_some()
                && parse_timestamp(b.split_whitespace().next().unwrap_or("")).is_some()
        })
    };
    if text.lines().any(is_cue_timing) {
        let cues = text
            .lines()
            .map(|l| {
                if is_cue_timing(l) {
                    format!("\n{l}")
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        return (cues, "vtt");
    }

    let speaker_and_time = |line: &str| {
        let (name, time) = line.rsplit_once(char::is_whitespace)?;
        let name = name.trim();
        (time.contains(':') && !name.is_empty() && parse_timestamp(time).is_some())
            .then(|| (name.to_string(), time.to_string()))
    };
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    // Only a transcript where these headers are about every other line; notes that merely
    // end in a time ("Call moved to 10:30") are left alone
    let headers = lines
        .iter()
        .filter(|l| speaker_and_time(l).is_some())
        .count();
    if headers < 2 || headers * 3 < lines.len() {
        return (lines.join("\n"), "txt");
    }
    let mut out = Vec::new();
    let mut rest = lines.into_iter().peekable();
    while let Some(line) = rest.next() {
        match (speaker_and_time(line), rest.peek()) {
            (Some((name, time)), Some(words)) if speaker_and_time(words).is_none() => {
                out.push(format!("{time} {name}: {words}"));
                rest.next();
            }
            _ => out.push(line.to_string()),
        }
    }
    (out.join("\n"), "txt")
}

fn unescape_xml(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    let entity = regex::Regex::new(r"&(amp|lt|gt|quot|apos|#x[0-9a-fA-F]+|#[0-9]+);").unwrap();
    entity
        .replace_all(text, |c: &regex::Captures| {
            let decoded = match &c[1] {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                n if n.starts_with("#x") => u32::from_str_radix(&n[2..], 16)
                    .ok()
                    .and_then(char::from_u32),
                n => n[1..].parse().ok().and_then(char::from_u32),
            };
            decoded.map(String::from).unwrap_or_default()
        })
        .into_owned()
}

async fn import_transcript_file<R: Runtime>(
    app: &AppHandle<R>,
    path: &Path,
) -> Result<ImportResult> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();
    if !TRANSCRIPT_EXTENSIONS.contains(&extension.as_str()) {
        return Err(anyhow!("Unsupported transcript format: .{}", extension));
    }

    let (content, format) = if extension == "docx" {
        docx_transcript(&docx_text(path)?)
    } else {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow!("Could not read {} as UTF-8 text: {}", path.display(), e))?;
        (text, extension.as_str())
    };
    let segments = to_segments(parse_transcript(&content, format));
    if segments.is_empty() {
        return Err(anyhow!("No transcript text found in {}", path.display()));
    }

    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported transcript")
        .to_string();
    let duration_seconds = segments
        .iter()
        .filter_map(|s| s.audio_end_time)
        .fold(0.0, f64::max);

    let meeting_folder = create_meeting_folder(&get_default_recordings_folder(), &title, false)?;
    let app_state = app
        .try_state::<AppState>()
        .ok_or_else(|| anyhow!("App state not available"))?;
    let meeting_id = create_meeting_with_transcripts(
        app_state.db_manager.pool(),
        &title,
        &segments,
        meeting_folder.to_string_lossy().to_string(),
    )
    .await?;

    if let Err(e) = write_transcripts_json(&meeting_folder, &segments) {
        warn!("Failed to write transcripts.json: {}", e);
    }
    if let Err(e) = write_import_metadata(
        &meeting_folder,
        &meeting_id,
        &title,
        duration_seconds,
        "",
        "transcript-import",
    ) {
        warn!("Failed to write metadata.json: {}", e);
    }

    info!(
        "Imported transcript as {} ({} segments)",
        meeting_id,
        segments.len()
    );
    Ok(ImportResult {
        meeting_id,
        title,
        segments_count: segments.len(),
        duration_seconds,
    })
}

/// Pick a transcript file and import it as a new meeting. Returns None if the user cancels the dialog.
#[tauri::command]
pub async fn import_transcript_command<R: Runtime>(
    app: AppHandle<R>,
) -> Result<Option<ImportResult>, String> {
    let app_clone = app.clone();
    let picked = tokio::task::spawn_blocking(move || {
        app_clone
            .dialog()
            .file()
            .add_filter("Transcripts", TRANSCRIPT_EXTENSIONS)
            .blocking_pick_file()
    })
    .await
    .map_err(|e| format!("File dialog task failed: {}", e))?;

    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = std::path::PathBuf::from(picked.to_string());
    import_transcript_file(&app, &path)
        .await
        .map(Some)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_srt_with_times() {
        // Second separator line holds whitespace, as some exporters write it
        let srt = "1\r\n00:00:01,500 --> 00:00:04,000\r\nHello there\r\nsecond line\r\n  \r\n2\r\n00:01:00,000 --> 00:01:02,250\r\nBye\r\n";
        let out = parse_transcript(srt, "srt");
        assert_eq!(out.len(), 2);
        assert_eq!(
            out[0],
            ("Hello there second line".to_string(), Some(1.5), Some(4.0))
        );
        assert_eq!(out[1], ("Bye".to_string(), Some(60.0), Some(62.25)));
    }

    #[test]
    fn parses_vtt_with_speakers_and_short_times() {
        let vtt = "\u{feff}WEBVTT\n\nNOTE exported from Teams\n\ncue-1\n00:05.000 --> 00:07.500 align:start\n<v Saleh Ahmed>Let's <b>start</b></v>\n";
        let out = parse_transcript(vtt, "vtt");
        assert_eq!(
            out,
            vec![("Saleh Ahmed: Let's start".to_string(), Some(5.0), Some(7.5))]
        );
    }

    #[test]
    fn parses_plain_text_per_line() {
        let out = parse_transcript("Alice: hi\n\n  Bob: hello  \n", "txt");
        assert_eq!(
            out,
            vec![
                ("Alice: hi".to_string(), None, None),
                ("Bob: hello".to_string(), None, None)
            ]
        );
    }

    #[test]
    fn parses_leading_timestamps_in_meet_style_text() {
        // "- First point" is an untimed list line inside Priya's turn: it joins that line
        let txt = "# Meeting Transcription\nDuration: 56 minutes\n## Transcript\n00:00 Alex Doe: Hello!\n00:05 Priya Rao: Hi, thanks.\n- First point\n[1:02:03] Alex Doe: Wrapping up.\n";
        assert_eq!(
            parse_transcript(txt, "txt"),
            vec![
                ("# Meeting Transcription".to_string(), None, None),
                ("Duration: 56 minutes".to_string(), None, None),
                ("## Transcript".to_string(), None, None),
                ("Alex Doe: Hello!".to_string(), Some(0.0), Some(5.0)),
                (
                    "Priya Rao: Hi, thanks. - First point".to_string(),
                    Some(5.0),
                    Some(3723.0)
                ),
                (
                    "Alex Doe: Wrapping up.".to_string(),
                    Some(3723.0),
                    Some(3723.0)
                ),
            ]
        );
    }

    #[test]
    fn keeps_notes_as_plain_text_when_few_lines_start_with_a_time() {
        let txt =
            "Client call notes\n10:30 call moved to Friday\nNeed CSV export\nFix slow login\n";
        assert!(parse_transcript(txt, "txt")
            .iter()
            .all(|(_, start, end)| start.is_none() && end.is_none()));
        assert_eq!(
            parse_transcript(txt, "txt")[1].0,
            "10:30 call moved to Friday"
        );
    }

    #[test]
    fn reads_docx_paragraphs_and_entities() {
        let xml = r#"<w:document><w:body>
            <w:p w:rsidR="1"><w:pPr><w:jc w:val="left"/></w:pPr>
              <w:r><w:t>Saleh Ahmed</w:t></w:r><w:r><w:tab/><w:t xml:space="preserve">0:03</w:t></w:r></w:p>
            <w:p/>
            <w:p><w:r><w:t>Scope &amp; budget &lt;agreed&gt; &#8212; caf&#xE9;</w:t></w:r></w:p>
            <w:tbl><w:tr><w:tc><w:p><w:r><w:t>Cell</w:t></w:r><w:r><w:br/><w:t>next</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
        </w:body></w:document>"#;
        assert_eq!(
            docx_xml_to_text(xml),
            "Saleh Ahmed 0:03\n\nScope & budget <agreed> \u{2014} caf\u{e9}\nCell\nnext"
        );
    }

    #[test]
    fn reads_a_docx_file_as_text() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Client call.docx");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        zip.start_file(
            "word/document.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(
            b"<w:document><w:body><w:p><w:r><w:t>00:05 Alex: hi</w:t></w:r></w:p>\
              <w:p><w:r><w:t>00:09 Sam: hello</w:t></w:r></w:p></w:body></w:document>",
        )
        .unwrap();
        zip.finish().unwrap();

        let text = docx_text(&path).unwrap();
        assert_eq!(
            parse_transcript(&text, "txt"),
            vec![
                ("Alex: hi".to_string(), Some(5.0), Some(9.0)),
                ("Sam: hello".to_string(), Some(9.0), Some(9.0)),
            ]
        );
        assert!(docx_text(&dir.path().join("missing.docx")).is_err());
    }

    #[test]
    fn docx_cue_lines_become_vtt_cues() {
        let text = "0:0:0.0 --> 0:0:5.2\nSaleh Ahmed\nHello\n0:0:5.2 --> 0:0:9.0\nPriya\nHi";
        let (content, format) = docx_transcript(text);
        assert_eq!(
            parse_transcript(&content, format),
            vec![
                ("Saleh Ahmed Hello".to_string(), Some(0.0), Some(5.2)),
                ("Priya Hi".to_string(), Some(5.2), Some(9.0)),
            ]
        );
        // An arrow in ordinary text is not a cue
        assert_eq!(docx_transcript("Plan A --> Plan B\nAgreed").1, "txt");
    }

    #[test]
    fn docx_speaker_time_headers_keep_time_and_speaker() {
        let text = "Transcript\nSaleh Ahmed   0:03\nHello everyone.\nPriya 1:10\nHi";
        let (content, format) = docx_transcript(text);
        assert_eq!(
            parse_transcript(&content, format),
            vec![
                ("Transcript".to_string(), None, None),
                (
                    "Saleh Ahmed: Hello everyone.".to_string(),
                    Some(3.0),
                    Some(70.0)
                ),
                ("Priya: Hi".to_string(), Some(70.0), Some(70.0)),
            ]
        );
        // Notes that happen to end in a time are left as they are
        let notes = "Agenda\nCall moved to 10:30\nBudget agreed";
        assert_eq!(docx_transcript(notes), (notes.to_string(), "txt"));
    }

    #[test]
    fn segments_get_duration_only_with_both_times() {
        let segs = to_segments(vec![
            ("a".into(), Some(1.0), Some(3.5)),
            ("b".into(), None, None),
        ]);
        assert_eq!(segs[0].duration, Some(2.5));
        assert_eq!(segs[1].duration, None);
        assert!(!segs[0].timestamp.is_empty());
        assert_eq!(segs[1].timestamp, "");
    }

    #[tokio::test]
    async fn imported_transcript_is_readable_as_a_meeting() {
        use crate::database::repositories::meeting::MeetingsRepository;

        // One connection: each in-memory SQLite connection is a separate database
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let product =
            crate::database::repositories::project::ProjectsRepository::create(&pool, "Product")
                .await
                .unwrap();

        let srt = "1\n00:00:02,000 --> 00:00:07,500\nAlex: Hello\n\n2\n00:00:08,000 --> 00:00:15,000\nPriya: Hi\n";
        let segments = to_segments(parse_transcript(srt, "srt"));
        let meeting_id =
            create_meeting_with_transcripts(&pool, "Product sync", &segments, String::new())
                .await
                .unwrap();

        let meeting = MeetingsRepository::get_meeting(&pool, &meeting_id)
            .await
            .unwrap()
            .expect("imported meeting should exist");
        assert_eq!(meeting.title, "Product sync");
        let project_id: Option<String> =
            sqlx::query_scalar("SELECT project_id FROM meetings WHERE id = ?")
                .bind(&meeting_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            project_id,
            Some(product.id),
            "filed into the project named in its title"
        );
        let mut rows: Vec<_> = meeting
            .transcripts
            .iter()
            .map(|t| (t.text.clone(), t.audio_start_time))
            .collect();
        rows.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        assert_eq!(
            rows,
            vec![
                ("Alex: Hello".to_string(), Some(2.0)),
                ("Priya: Hi".to_string(), Some(8.0))
            ]
        );
    }
}
