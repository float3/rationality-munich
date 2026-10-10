// Writes public/downloads/: one .ics per event in the snapshot, plus one of
// the lot. Run after `pnpm snapshot`.
import fs from 'node:fs/promises';
import path from 'node:path';
import { events, SNAPSHOT, SNAPSHOT_LABEL } from '../lib/content.ts';

const root = path.resolve(import.meta.dirname, '..');

// The 3.7 MB original is not in the repository; only the webp it produces is.
// Drop the full-size photograph next to it to regenerate (see README).
const photo = path.join(root, 'public/images/munich.jpg');
if (await fs.stat(photo).then(() => true, () => false)) {
  const { default: sharp } = await import('sharp');
  await sharp(photo)
    .resize({ width: 1600, withoutEnlargement: true })
    .webp({ quality: 82 })
    .toFile(path.join(root, 'public/images/munich.webp'));
}

const escapeText = value =>
  value.replaceAll('\\', '\\\\').replaceAll('\n', '\\n').replaceAll(';', '\\;').replaceAll(',', '\\,');
const compactDate = value => value.replaceAll('-', '').replaceAll(':', '');
const stamp = `${compactDate(SNAPSHOT)}T120000Z`;

function fold(line) {
  let result = '', current = '', bytes = 0;
  for (const char of line) {
    const size = Buffer.byteLength(char);
    if (bytes + size > 74) { result += current + '\r\n '; current = ''; bytes = 1 }
    current += char; bytes += size;
  }
  return result + current;
}

function calendar(items) {
  const lines = ['BEGIN:VCALENDAR', 'VERSION:2.0', 'PRODID:-//Rationality Munich Demo//Events//EN', 'CALSCALE:GREGORIAN', 'METHOD:PUBLISH', `X-WR-CALNAME:Munich events — snapshot of ${SNAPSHOT_LABEL}`, 'X-WR-TIMEZONE:Europe/Berlin'];
  for (const event of items) {
    lines.push('BEGIN:VEVENT', `UID:demo-${event.id}@rationality-munich.com`, `DTSTAMP:${stamp}`, `DTSTART:${compactDate(event.start)}`);
    if (event.end) lines.push(`DTEND:${compactDate(event.end)}`);
    lines.push(`SUMMARY:${escapeText(event.title)}`, `LOCATION:${escapeText(event.place)}`, `DESCRIPTION:${escapeText(event.summary + '\nSaved ' + SNAPSHOT_LABEL + '. Check the original announcement for changes: ' + event.source)}`, `URL:${event.source}`, 'END:VEVENT');
  }
  lines.push('END:VCALENDAR');
  return lines.map(fold).join('\r\n') + '\r\n';
}

// A fresh snapshot renames every event, so start from an empty directory
// rather than leaving the last one's files behind.
const downloads = path.join(root, 'public/downloads');
await fs.rm(downloads, { recursive: true, force: true });
await fs.mkdir(downloads, { recursive: true });
for (const event of events) await fs.writeFile(path.join(downloads, `${event.id}.ics`), calendar([event]));
await fs.writeFile(path.join(downloads, 'munich-events.ics'), calendar(events));
console.log(`Prepared ${events.length} calendar downloads, and one of the lot.`);
