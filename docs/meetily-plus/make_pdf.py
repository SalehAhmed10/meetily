"""Render BUILD_SHEET.md to BUILD_SHEET.pdf with headless Chrome/Edge. No pip packages needed.

Supports the Markdown this sheet uses: #/##/### headings, | tables |, - lists, > quotes,
**bold**, `code`, [links](url). Usage: python docs/meetily-plus/make_pdf.py
"""
import html
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "BUILD_SHEET.md"
TARGET = HERE / "BUILD_SHEET.pdf"
BROWSERS = [
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
]
CSS = """
body { font: 10.5pt/1.45 'Segoe UI', Arial, sans-serif; color: #1f2937; margin: 0; }
h1 { font-size: 20pt; margin: 0 0 6pt; } h2 { font-size: 13pt; margin: 16pt 0 6pt; border-bottom: 1px solid #e5e7eb; padding-bottom: 3pt; }
table { border-collapse: collapse; width: 100%; margin: 6pt 0; page-break-inside: auto; }
th, td { border: 1px solid #d1d5db; padding: 4pt 6pt; vertical-align: top; text-align: left; }
th { background: #f3f4f6; } tr { page-break-inside: avoid; }
code { font-family: Consolas, monospace; font-size: 9pt; background: #f3f4f6; padding: 0 2pt; border-radius: 2pt; }
blockquote { margin: 8pt 0; padding: 4pt 10pt; border-left: 3px solid #3b82f6; background: #eff6ff; }
ul { margin: 4pt 0; padding-left: 16pt; } a { color: #1d4ed8; }
@page { size: A4 landscape; margin: 14mm; }
"""


def inline(text: str) -> str:
    out = html.escape(text, quote=False)
    out = re.sub(r"`([^`]+)`", r"<code>\1</code>", out)
    out = re.sub(r"\*\*([^*]+)\*\*", r"<strong>\1</strong>", out)
    return re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', out)


def to_html(markdown: str) -> str:
    parts, table, items = [], [], []

    def flush():
        if table:
            rows = [r for r in table if not re.fullmatch(r"\|[\s\-|:]+\|", r)]
            cells = [[inline(c.strip()) for c in r.strip("|").split("|")] for r in rows]
            head = "".join(f"<th>{c}</th>" for c in cells[0])
            body = "".join("<tr>" + "".join(f"<td>{c}</td>" for c in r) + "</tr>" for r in cells[1:])
            parts.append(f"<table><thead><tr>{head}</tr></thead><tbody>{body}</tbody></table>")
            table.clear()
        if items:
            parts.append("<ul>" + "".join(f"<li>{i}</li>" for i in items) + "</ul>")
            items.clear()

    for line in markdown.splitlines():
        stripped = line.strip()
        if stripped.startswith("|"):
            table.append(stripped)
            continue
        if stripped.startswith("- "):
            items.append(inline(stripped[2:]))
            continue
        flush()
        if not stripped:
            continue
        heading = re.match(r"(#{1,3}) (.*)", stripped)
        if heading:
            level = len(heading.group(1))
            parts.append(f"<h{level}>{inline(heading.group(2))}</h{level}>")
        elif stripped.startswith("> "):
            parts.append(f"<blockquote>{inline(stripped[2:])}</blockquote>")
        else:
            parts.append(f"<p>{inline(stripped)}</p>")
    flush()
    return "\n".join(parts)


def main() -> int:
    browser = next((b for b in BROWSERS if Path(b).exists()), None)
    if not browser:
        print("Chrome or Edge not found", file=sys.stderr)
        return 1
    page = f"<!doctype html><meta charset='utf-8'><style>{CSS}</style>{to_html(SOURCE.read_text(encoding='utf-8'))}"
    with tempfile.TemporaryDirectory() as tmp:
        html_path = Path(tmp) / "build_sheet.html"
        html_path.write_text(page, encoding="utf-8")
        subprocess.run(
            [browser, "--headless=new", "--disable-gpu", "--no-pdf-header-footer",
             f"--user-data-dir={tmp}", f"--print-to-pdf={TARGET}", html_path.as_uri()],
            check=True, capture_output=True, timeout=120,
        )
    print(f"Wrote {TARGET} ({TARGET.stat().st_size // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
