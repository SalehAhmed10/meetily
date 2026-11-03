import { describe, expect, test } from 'bun:test';
import { parseEmail } from '../../src/lib/email-parse';

describe('parseEmail', () => {
  test('reads a multipart .eml: headers, encoded subject, quoted-printable plain part', () => {
    const eml = [
      'From: Daniel <daniel@remotehaven.com>',
      'Subject: =?UTF-8?B?Q29uZ3JhdHVsYXRpb25zIOKAkyBSb3VuZCAy?=',
      'Date: Wed, 05 Nov 2026 09:30:00 +0000',
      'Content-Type: multipart/alternative;',
      ' boundary="b1"',
      '',
      '--b1',
      'Content-Type: text/plain; charset="utf-8"',
      'Content-Transfer-Encoding: quoted-printable',
      '',
      'Hi Saleh,',
      '',
      'You=E2=80=99re through to round 2 =3D next week. Please prepare a short=',
      ' demo.',
      '--b1',
      'Content-Type: text/html; charset="utf-8"',
      '',
      '<p>HTML version</p>',
      '--b1--',
    ].join('\r\n');
    const email = parseEmail(eml);
    expect(email.subject).toBe('Congratulations – Round 2');
    expect(email.from).toBe('Daniel <daniel@remotehaven.com>');
    expect(email.date?.toISOString()).toBe('2026-11-05T09:30:00.000Z');
    expect(email.body).toBe("Hi Saleh,\n\nYou’re through to round 2 = next week. Please prepare a short demo.");
  });

  test('falls back to the HTML part and base64', () => {
    const eml = [
      'Subject: Decision',
      'Content-Type: text/html; charset=utf-8',
      'Content-Transfer-Encoding: base64',
      '',
      btoa('<p>Thanks &amp; regards</p><p>Next step</p>'),
    ].join('\n');
    expect(parseEmail(eml).body).toBe('Thanks & regards\n\nNext step');
  });

  test('reads the header block Outlook puts on copied mail', () => {
    const pasted = 'From: Priya\nSent: 6 November 2026 14:05\nSubject: Phase 2 kickoff\n\nHi,\n\nLet us start phase 2.';
    const email = parseEmail(pasted);
    expect(email.subject).toBe('Phase 2 kickoff');
    expect(email.date?.getDate()).toBe(6);
    expect(email.body).toBe('Hi,\n\nLet us start phase 2.');
  });

  test('plain pasted text is all body', () => {
    const email = parseEmail('Hi Saleh,\n\nUnfortunately we will not proceed.');
    expect(email).toEqual({ subject: '', from: '', date: null, body: 'Hi Saleh,\n\nUnfortunately we will not proceed.' });
  });

  test('a first line that only looks like a header is still body', () => {
    expect(parseEmail('Note: call me\nabout the contract').body).toBe('Note: call me\nabout the contract');
    expect(parseEmail('Update: call moved to 3pm\n\nHi all,').body).toBe('Update: call moved to 3pm\n\nHi all,');
  });

  test('skips attachments and decodes HTML entities', () => {
    const eml = [
      'Subject: Interview',
      'Content-Type: multipart/mixed; boundary="m"',
      '',
      '--m',
      'Content-Type: text/html; charset=utf-8',
      '',
      '<p>Don&#8217;t forget &mdash; round&nbsp;2</p>',
      '--m',
      'Content-Type: text/calendar; charset=utf-8',
      'Content-Disposition: attachment; filename="invite.ics"',
      '',
      'BEGIN:VCALENDAR',
      '--m--',
    ].join('\r\n');
    expect(parseEmail(eml).body).toBe('Don’t forget — round 2');
  });
});
