import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {events} from '../lib/content.ts';
import {topics,validateSignup} from '../lib/subscription.ts';
assert.equal(events.length,5);
assert.equal(new Set(events.map(e=>e.id)).size,events.length);
for(const event of events){assert.ok(event.source.startsWith('https://'));assert.equal(new Intl.DateTimeFormat('en-CA',{timeZone:'Europe/Berlin'}).format(new Date(event.start)),event.date);if(event.end)assert.ok(Date.parse(event.end)>Date.parse(event.start));const ics=await fs.readFile(new URL(`../public/downloads/${event.id}.ics`,import.meta.url),'utf8');assert.ok(ics.includes('BEGIN:VEVENT'));assert.ok(ics.includes(`UID:concept-${event.id}`));assert.equal(ics.includes('DTEND:'),!!event.end);for(const line of ics.split('\r\n'))assert.ok(Buffer.byteLength(line)<=75,'Calendar lines must be folded at 75 octets')}
assert.equal(validateSignup('person@example.com',topics.map(t=>t.id)),'');
assert.equal(validateSignup(' person@example.com ',['ea']),'');
assert.ok(validateSignup('invalid',['ea']));
assert.ok(validateSignup('person@example.com',[]));
assert.ok(validateSignup('person@example.com',['invented']));
assert.ok(validateSignup('',['ea']));
console.log('Passed event consistency, calendar downloads, timezone, line folding, and signup validation checks.');
