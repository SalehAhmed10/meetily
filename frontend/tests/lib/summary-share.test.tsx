import { describe, expect, test } from 'bun:test';
import {
  followUpEmailDraft,
  mailtoUrl,
  markdownToPlainText,
  summaryHtmlDocument,
} from '../../src/lib/summary-share';

const summary = `**Summary**
Scope agreed.

**Action Items**

| Owner | Task |
| --- | --- |
| Saleh | Send scope |

**Follow-up Email Draft**
Hi Priya,

Thanks for today. Phase one is login, catalogue & checkout.

## Notes
- not part of the email`;

describe('summaryHtmlDocument', () => {
  test('renders tables and escapes the title', () => {
    const html = summaryHtmlDocument('Acme <scope> call', '3 Nov 2026', summary);
    expect(html).toContain('<title>Acme &lt;scope&gt; call</title>');
    expect(html).toContain('<table>');
    expect(html).toContain('<td>Send scope</td>');
    expect(html).toContain('<strong>Summary</strong>');
  });

  test('drops images so nothing is fetched while printing', () => {
    const html = summaryHtmlDocument('t', 'd', 'Hi ![x](http://example.com/leak?data=1)');
    expect(html).not.toContain('example.com');
    expect(html).not.toContain('<img');
  });
});

describe('followUpEmailDraft', () => {
  test('takes the draft section up to the next heading', () => {
    expect(followUpEmailDraft(summary)).toBe('Hi Priya,\n\nThanks for today. Phase one is login, catalogue & checkout.');
  });
  test('returns null when there is no draft', () => {
    expect(followUpEmailDraft('**Summary**\nNothing to send.')).toBeNull();
  });
});

describe('mailtoUrl', () => {
  test('encodes & so the body is not cut off', () => {
    const url = mailtoUrl('Recap: Acme', 'login & checkout');
    expect(url).toBe('mailto:?subject=Recap%3A%20Acme&body=login%20%26%20checkout');
  });
  test('cuts a long body at a line break and says the full text was copied', () => {
    const body = Array.from({ length: 100 }, (_, i) => `line ${i} of the summary`).join('\n');
    const decoded = decodeURIComponent(mailtoUrl('s', body, 300).split('&body=')[1]);
    expect(decoded.length).toBeLessThan(400);
    expect(decoded).toContain('[Full text copied to your clipboard');
  });
});

test('markdownToPlainText drops heading and bold marks', () => {
  expect(markdownToPlainText('## Decisions\n**Budget** is \\$4,000')).toBe('Decisions\nBudget is $4,000');
});
