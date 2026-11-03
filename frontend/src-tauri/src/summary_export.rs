// Export a meeting summary to PDF: the page renders the summary as a standalone HTML document,
// the user picks where to save, and a headless Edge/Chrome prints it.

use std::path::{Path, PathBuf};
use tauri::{AppHandle, Runtime};
use tauri_plugin_dialog::DialogExt;

// ponytail: Windows/macOS install paths of Edge and Chrome only; add a Linux lookup if needed
const BROWSERS: &[&str] = &[
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
];

/// File name without characters Windows refuses
fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if r#"<>:"/\|?*"#.contains(c) || c.is_control() {
                ' '
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        "Meeting summary".to_string()
    } else {
        cleaned
    }
}

fn print_to_pdf(browser: &Path, html: &str, target: &Path) -> Result<(), String> {
    let work = tempfile::tempdir().map_err(|e| format!("Could not create a temp folder: {e}"))?;
    let page = work.path().join("summary.html");
    std::fs::write(&page, html).map_err(|e| format!("Could not write the page: {e}"))?;
    let url = url::Url::from_file_path(&page).map_err(|_| "Bad temp path".to_string())?;
    let output = std::process::Command::new(browser)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-pdf-header-footer")
        .arg(format!(
            "--user-data-dir={}",
            work.path().join("profile").display()
        ))
        .arg(format!("--print-to-pdf={}", target.display()))
        .arg(url.as_str())
        .output()
        .map_err(|e| format!("Could not start {}: {e}", browser.display()))?;
    if target.exists() {
        Ok(())
    } else {
        Err(format!(
            "The PDF was not created ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Asks where to save, then writes the summary PDF there. Returns the saved path, or None
/// when the user cancels the dialog.
#[tauri::command]
pub async fn export_summary_pdf<R: Runtime>(
    app: AppHandle<R>,
    html: String,
    file_name: String,
) -> Result<Option<String>, String> {
    let browser = BROWSERS
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .ok_or("PDF export needs Microsoft Edge or Google Chrome installed")?;
    let name = format!("{}.pdf", safe_file_name(&file_name));
    let picked = tokio::task::spawn_blocking(move || {
        app.dialog()
            .file()
            .set_file_name(&name)
            .add_filter("PDF", &["pdf"])
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("File dialog task failed: {e}"))?;
    let Some(picked) = picked else {
        return Ok(None);
    };
    let target = picked
        .into_path()
        .map_err(|e| format!("Bad save location: {e}"))?;
    let saved = target.clone();
    tokio::task::spawn_blocking(move || print_to_pdf(&browser, &html, &target))
        .await
        .map_err(|e| format!("PDF task failed: {e}"))??;
    Ok(Some(saved.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_lose_characters_windows_refuses() {
        assert_eq!(
            safe_file_name("Acme: scope / phase 1?"),
            "Acme scope phase 1"
        );
        assert_eq!(safe_file_name("  "), "Meeting summary");
    }

    /// Renders a real PDF when Edge or Chrome is installed (skipped otherwise)
    #[test]
    fn prints_html_to_a_pdf_file() {
        let Some(browser) = BROWSERS.iter().map(PathBuf::from).find(|p| p.exists()) else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("out.pdf");
        print_to_pdf(&browser, "<h1>Acme</h1><p>Scope agreed.</p>", &target).unwrap();
        let bytes = std::fs::read(&target).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }
}
