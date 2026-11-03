/** What the Add email form needs from a saved .eml file or a pasted email */
export interface ParsedEmail {
  subject: string;
  from: string;
  date: Date | null;
  body: string;
}

type Headers = Map<string, string>;

/** Leading "Name: value" lines (with folded continuation lines) up to the first blank line */
function splitHeaders(text: string): { headers: Headers; rest: string } {
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const headers: Headers = new Map();
  let last: string | null = null;
  let i = 0;
  for (; i < lines.length; i++) {
    const line = lines[i];
    if (line.trim() === '') break;
    if (/^[ \t]/.test(line) && last) {
      headers.set(last, `${headers.get(last)} ${line.trim()}`);
      continue;
    }
    const match = /^([A-Za-z][A-Za-z0-9-]*):\s*(.*)$/.exec(line);
    if (!match) break;
    last = match[1].toLowerCase();
    headers.set(last, match[2]);
  }
  // Not a header block: an ordinary pasted email starts with text, and a first line like
  // "Update: call moved" is not a header unless it names a mail field
  const mailField = ['from', 'subject', 'date', 'sent', 'content-type'].some(name => headers.has(name));
  if (!mailField || (i < lines.length && lines[i].trim() !== '')) {
    return { headers: new Map(), rest: text.replace(/\r\n/g, '\n') };
  }
  return { headers, rest: lines.slice(i + 1).join('\n') };
}

function decodeBytes(bytes: Uint8Array, charset: string): string {
  try {
    return new TextDecoder(charset || 'utf-8').decode(bytes);
  } catch {
    return new TextDecoder('utf-8').decode(bytes);
  }
}

function decodeQuotedPrintable(text: string, charset: string): string {
  const joined = text.replace(/=\r?\n/g, '');
  const bytes: number[] = [];
  for (let i = 0; i < joined.length; i++) {
    const hex = joined.slice(i + 1, i + 3);
    if (joined[i] === '=' && /^[0-9A-Fa-f]{2}$/.test(hex)) {
      bytes.push(parseInt(hex, 16));
      i += 2;
    } else {
      bytes.push(...new TextEncoder().encode(joined[i]));
    }
  }
  return decodeBytes(new Uint8Array(bytes), charset);
}

function decodeBase64(text: string, charset: string): string {
  try {
    const binary = atob(text.replace(/\s+/g, ''));
    return decodeBytes(Uint8Array.from(binary, c => c.charCodeAt(0)), charset);
  } catch {
    return text;
  }
}

/** "=?UTF-8?B?...?=" / "=?utf-8?Q?...?=" words in a header */
function decodeHeader(value: string): string {
  return value
    .replace(/\?=\s+=\?/g, '?==?')
    .replace(/=\?([^?]+)\?([BbQq])\?([^?]*)\?=/g, (_, charset: string, kind: string, text: string) =>
      kind.toUpperCase() === 'B'
        ? decodeBase64(text, charset)
        : decodeQuotedPrintable(text.replace(/_/g, ' '), charset),
    );
}

const param = (value: string, name: string) =>
  new RegExp(`${name}="?([^";]+)"?`, 'i').exec(value)?.[1] ?? '';

function htmlToText(html: string): string {
  return html
    .replace(/<(style|script)[\s\S]*?<\/\1>/gi, '')
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<\/(p|div|li|tr|h[1-6])>/gi, '\n\n')
    .replace(/<[^>]+>/g, '')
    .replace(/&#(\d+);/g, (_, n: string) => String.fromCodePoint(Number(n)))
    .replace(/&#x([0-9a-f]+);/gi, (_, n: string) => String.fromCodePoint(parseInt(n, 16)))
    .replace(/&([a-z]+);/gi, (entity, name: string) => NAMED_ENTITIES[name.toLowerCase()] ?? entity);
}

const NAMED_ENTITIES: Record<string, string> = {
  nbsp: ' ', amp: '&', lt: '<', gt: '>', quot: '"', apos: "'",
  rsquo: '’', lsquo: '‘', rdquo: '”', ldquo: '“', mdash: '—', ndash: '–', hellip: '…', bull: '•',
};

/** The readable text of a MIME entity: text/plain preferred, HTML as a fallback */
function entityText(headers: Headers, body: string): { text: string; html: boolean } | null {
  const type = (headers.get('content-type') ?? 'text/plain').toLowerCase();
  if (type.startsWith('multipart/')) {
    const boundary = param(headers.get('content-type') ?? '', 'boundary');
    if (!boundary) return null;
    const parts = body.split(`--${boundary}`).slice(1).filter(p => !p.startsWith('--'));
    const found = parts
      .map(part => {
        const { headers: h, rest } = splitHeaders(part.replace(/^\r?\n/, ''));
        return entityText(h, rest);
      })
      .filter((t): t is { text: string; html: boolean } => t !== null);
    return found.find(t => !t.html) ?? found[0] ?? null;
  }
  // Attachments (a .ics invite, a .csv) are not the message
  if (/^\s*attachment/i.test(headers.get('content-disposition') ?? '')) return null;
  if (!type.startsWith('text/plain') && !type.startsWith('text/html')) return null;
  const charset = param(headers.get('content-type') ?? '', 'charset');
  const encoding = (headers.get('content-transfer-encoding') ?? '').toLowerCase();
  const decoded =
    encoding === 'quoted-printable' ? decodeQuotedPrintable(body, charset)
      : encoding === 'base64' ? decodeBase64(body, charset)
        : body;
  const html = type.startsWith('text/html');
  return { text: html ? htmlToText(decoded) : decoded, html };
}

/**
 * Reads a saved .eml or a pasted email. Header fields fill subject, sender and date when
 * present (raw .eml headers, or the "From:/Sent:/Subject:" block Outlook puts on copied mail);
 * otherwise the whole text is the body.
 */
export function parseEmail(raw: string): ParsedEmail {
  const { headers, rest } = splitHeaders(raw.trim());
  const content = headers.has('content-type') ? entityText(headers, rest) : null;
  const body = (content ? content.text : rest)
    .replace(/\r\n/g, '\n')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim();
  const dateText = headers.get('date') ?? headers.get('sent');
  const date = dateText ? new Date(dateText) : null;
  return {
    subject: decodeHeader(headers.get('subject') ?? '').trim(),
    from: decodeHeader(headers.get('from') ?? '').trim(),
    date: date && !Number.isNaN(date.getTime()) ? date : null,
    body,
  };
}
