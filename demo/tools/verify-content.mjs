// The snapshot holds together: real links, dates that agree with their
// timestamps, a well-formed .ics per event, and a signup form that validates.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import { events, SNAPSHOT } from '../lib/content.ts';
import { topics, validateSignup } from '../lib/subscription.ts';

assert.ok(events.length > 0, 'the snapshot is empty');
assert.equal(new Set(events.map(e => e.id)).size, events.length, 'duplicate event ids');

const berlinDay = value =>
  new Intl.DateTimeFormat('en-CA', { timeZone: 'Europe/Berlin', year: 'numeric', month: '2-digit', day: '2-digit' }).format(new Date(value));

for (const event of events) {
  assert.ok(event.source.startsWith('https://'), `${event.id}: no source link`);
  assert.equal(berlinDay(event.start), event.date, `${event.id}: date and start disagree`);
  // Nothing in the past: the snapshot is of what is still to come.
  assert.ok(event.date >= SNAPSHOT, `${event.id}: already over on ${SNAPSHOT}`);
  if (event.end) assert.ok(Date.parse(event.end) > Date.parse(event.start), `${event.id}: ends before it starts`);
  const ics = await fs.readFile(new URL(`../public/downloads/${event.id}.ics`, import.meta.url), 'utf8');
  assert.ok(ics.includes('BEGIN:VEVENT'));
  assert.ok(ics.includes(`UID:demo-${event.id}`));
  assert.equal(ics.includes('DTEND:'), !!event.end);
  for (const line of ics.split('\r\n')) assert.ok(Buffer.byteLength(line) <= 75, 'Calendar lines must be folded at 75 octets');
}

const stale = await fs.readdir(new URL('../public/downloads/', import.meta.url));
const expected = new Set([...events.map(e => `${e.id}.ics`), 'munich-events.ics']);
for (const file of stale) assert.ok(expected.has(file), `${file} is left over from an older snapshot`);

assert.equal(validateSignup('person@example.com', topics.map(t => t.id)), '');
assert.equal(validateSignup(' person@example.com ', ['ea']), '');
assert.ok(validateSignup('invalid', ['ea']));
assert.ok(validateSignup('person@example.com', []));
assert.ok(validateSignup('person@example.com', ['invented']));
assert.ok(validateSignup('', ['ea']));
console.log(`Passed ${events.length} events, their calendar downloads, timezone, line folding, and signup validation.`);
