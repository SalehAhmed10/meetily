import { renderToStaticMarkup } from 'react-dom/server';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

const escapeHtml = (text: string) =>
  text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

const PRINT_CSS = `
@page { size: A4; margin: 18mm 16mm; }
body { font: 10.5pt/1.5 "Segoe UI", system-ui, -apple-system, sans-serif; color: #111827; margin: 0; }
header { border-bottom: 1px solid #e5e7eb; padding-bottom: 8pt; margin-bottom: 14pt; }
header h1 { font-size: 18pt; line-height: 1.25; margin: 0 0 4pt; }
header p { color: #4b5563; margin: 0; font-size: 9.5pt; }
h1, h2, h3 { line-height: 1.3; break-after: avoid; }
h1 { font-size: 15pt; } h2 { font-size: 13pt; margin: 16pt 0 6pt; } h3 { font-size: 11.5pt; margin: 12pt 0 4pt; }
p, ul, ol { margin: 0 0 8pt; } ul, ol { padding-left: 16pt; } li { margin: 2pt 0; }
table { border-collapse: collapse; width: 100%; margin: 6pt 0 10pt; font-size: 9.5pt; }
th, td { border: 1px solid #d1d5db; padding: 4pt 6pt; text-align: left; vertical-align: top; }
th { background: #f3f4f6; } tr { break-inside: avoid; }
code { font-family: Consolas, monospace; font-size: 9pt; background: #f3f4f6; padding: 0 2pt; border-radius: 2pt; }
blockquote { margin: 8pt 0; padding: 2pt 10pt; border-left: 1px solid #93c5fd; color: #374151; }
a { color: #1d4ed8; }
`;

/** The summary as a standalone, print-ready HTML page (title, date, then the summary) */
export function summaryHtmlDocument(title: string, date: string, markdown: string): string {
  const body = renderToStaticMarkup(<ReactMarkdown remarkPlugins={[remarkGfm]}>{markdown}</ReactMarkdown>);
  return `<!doctype html><html><head><meta charset="utf-8"><title>${escapeHtml(title)}</title>
<style>${PRINT_CSS}</style></head><body>
<header><h1>${escapeHtml(title)}</h1><p>${escapeHtml(date)}</p></header>
${body}</body></html>`;
}

/**
 * The text of a "Follow-up Email Draft" section (a `#` heading or a bold line naming it), up
 * to the next heading; null when the summary has none.
 */
export function followUpEmailDraft(markdown: string): string | null {
  const lines = markdown.split(/\r?\n/);
  const isHeading = (line: string) => /^#{1,6}\s/.test(line.trim()) || /^\*\*[^*]+\*\*:?$/.test(line.trim());
  const start = lines.findIndex(l => isHeading(l) && /follow[- ]?up/i.test(l) && /email|mail/i.test(l));
  if (start < 0) return null;
  const rest = lines.slice(start + 1);
  const end = rest.findIndex(isHeading);
  const text = (end < 0 ? rest : rest.slice(0, end)).join('\n').trim();
  return text || null;
}

/** Markdown to text an email body can show: no heading marks, bold markers or escapes */
export function markdownToPlainText(markdown: string): string {
  return markdown
    .split(/\r?\n/)
    .map(line => line.replace(/^#{1,6}\s+/, '').replace(/\*\*/g, '').replace(/\\([\\`*_{}[\]()#+\-.!|$~<>=])/g, '$1'))
    .join('\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
}

/**
 * A mailto: link with subject and body. Mail apps refuse very long links, so a long body is
 * cut at a line break with a note to paste the full text (which the caller copies).
 */
export function mailtoUrl(subject: string, body: string, maxBody = 1500): string {
  let text = body;
  if (text.length > maxBody) {
    const cut = text.lastIndexOf('\n', maxBody);
    text = `${text.slice(0, cut > maxBody / 2 ? cut : maxBody).trimEnd()}\n\n[Full text copied to your clipboard: paste it here]`;
  }
  return `mailto:?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(text)}`;
}
