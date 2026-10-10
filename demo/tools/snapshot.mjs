// Regenerates lib/content.ts from the live calendar.
//
// The demo is a static export, so its events are a snapshot rather than a
// feed. This is what takes the snapshot: run it, then `pnpm publish-demo`.
// Two sources, because neither carries everything:
//
// - feed.xml decides which events are in (upcoming, the groups the hub shows
//   by default) and gives the place, the groups and the excerpt.
// - all.ics adds the end time and the organiser's own link.
//
// Everything else — the category and format labels the design uses — is read
// off the title and the excerpt here. They are decoration, not facts about
// the event; the organiser's announcement stays the source of truth and every
// event page links to it.
import fs from 'node:fs/promises';
import path from 'node:path';

const BASE = 'https://rationality-munich.com';
const HUB_HOST = 'rationality-munich.com';
/** `end_or_default` in the calendar invents this when an event has no end. */
const INVENTED_END_MS = 3 * 60 * 60 * 1000;

const root = path.resolve(import.meta.dirname, '..');

async function get(url) {
  const response = await fetch(url, { headers: { 'user-agent': 'rationality-munich-demo/snapshot' } });
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  return response.text();
}

// ---------------------------------------------------------------- the feeds

const unescapeXml = (s) =>
  s
    .replaceAll('&lt;', '<')
    .replaceAll('&gt;', '>')
    .replaceAll('&quot;', '"')
    .replaceAll('&#x27;', "'")
    .replaceAll('&apos;', "'")
    .replaceAll('&amp;', '&');

/** Upcoming events, in feed order, which is by start time. */
function parseAtom(xml) {
  return [...xml.matchAll(/<entry>([\s\S]*?)<\/entry>/g)].map(([, entry]) => {
    const field = (name) => unescapeXml(entry.match(new RegExp(`<${name}>([\\s\\S]*?)</${name}>`))[1]);
    const [head, ...rest] = field('summary').split('\n\n');
    // `<when> · <place, if any> · <groups>`; see feed.rs.
    const parts = head.split(' · ');
    return {
      id: field('id').split('#')[1],
      title: field('title'),
      start: new Date(field('updated')),
      place: parts.length > 2 ? parts.slice(1, -1).join(' · ') : '',
      group: parts[parts.length - 1],
      summary: rest.join('\n\n').trim(),
    };
  });
}

const unescapeIcs = (s) =>
  s
    .replaceAll(String.raw`\n`, '\n')
    .replaceAll(String.raw`\,`, ',')
    .replaceAll(String.raw`\;`, ';')
    .replaceAll('\\\\', '\\');

/** ICS timestamps are `20261012T164500Z`. */
const icsTime = (v) =>
  new Date(`${v.slice(0, 4)}-${v.slice(4, 6)}-${v.slice(6, 11)}:${v.slice(11, 13)}:${v.slice(13, 15)}Z`);

function parseIcs(text) {
  const lines = [];
  for (const line of text.split('\r\n')) {
    // Folded continuation lines start with a space.
    if (line.startsWith(' ') && lines.length > 0) lines[lines.length - 1] += line.slice(1);
    else lines.push(line);
  }
  const events = new Map();
  let current = null;
  for (const line of lines) {
    if (line === 'BEGIN:VEVENT') current = {};
    else if (line === 'END:VEVENT') {
      if (current?.UID) events.set(current.UID.split('@')[0], current);
      current = null;
    } else if (current) {
      const [name, value] = [line.slice(0, line.indexOf(':')), line.slice(line.indexOf(':') + 1)];
      if (name) current[name.split(';')[0]] = value;
    }
  }
  return events;
}

/**
 * The trailing `Label: url` lines of an ICS description, the hub's own link
 * dropped: those are the organisers' announcements.
 */
function sourcesOf(description) {
  const [, ...tail] = unescapeIcs(description).split('\n\n');
  return tail
    .join('\n')
    .split('\n')
    .map((line) => line.match(/^(.+?): (https?:\/\/\S+)$/))
    .filter((m) => m !== null && !m[2].includes(HUB_HOST))
    .map((m) => [m[1], m[2]]);
}

// ---------------------------------------------------------------- labelling

const PLATFORMS = [
  ['lesswrong.com', 'LessWrong'],
  ['forum.effectivealtruism.org', 'EA Forum'],
  ['meetup.com', 'Meetup'],
  ['luma.com', 'Luma'],
  ['lu.ma', 'Luma'],
  ['docs.google.com', 'Google Docs'],
  ['calendar.google.com', 'Google Calendar'],
  ['chat.whatsapp.com', 'WhatsApp'],
];

/** The site an announcement is on, so "Details & RSVP on …" reads sensibly. */
function platformOf(url, fallback) {
  const host = new URL(url).hostname;
  return PLATFORMS.find(([h]) => host.endsWith(h))?.[1] ?? fallback;
}

const CATEGORIES = [
  [/student club|ai safety/i, 'AI safety'],
  [/pauseai/i, 'AI policy'],
  [/effective altruism|ea munich/i, 'Effective altruism'],
  [/acx|lesswrong/i, 'Rationality'],
  [/philosophia/i, 'Philosophy'],
];

const FORMATS = [
  [/dinner/i, 'Dinner & discussion'],
  [/stammtisch/i, 'Regulars’ table'],
  [/solstice|ritual/i, 'Community gathering'],
  [/game/i, 'Thinking game'],
  [/reading group/i, 'Reading group'],
  [/meetup|social/i, 'Meetup'],
  [/workshop/i, 'Workshop'],
  [/discussion|discuss/i, 'Discussion'],
  [/talk|lecture/i, 'Talk'],
];

const first = (table, ...texts) => {
  for (const text of texts) {
    const hit = table.find(([pattern]) => pattern.test(text));
    if (hit) return hit[1];
  }
  return null;
};

// ---------------------------------------------------------------- Berlin time

const berlin = (date, options) =>
  new Intl.DateTimeFormat('en-GB', { timeZone: 'Europe/Berlin', ...options }).format(date);
const isoDay = (date) =>
  new Intl.DateTimeFormat('en-CA', {
    timeZone: 'Europe/Berlin',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).format(date);
const clock = (date) => berlin(date, { hour: '2-digit', minute: '2-digit', hour12: false });
const longDay = (date) => berlin(date, { day: 'numeric', month: 'long', year: 'numeric' });

// ---------------------------------------------------------------- generating

const quote = (s) => `'${s.replaceAll('\\', '\\\\').replaceAll("'", "\\'").replaceAll('\n', ' ')}'`;

function literal(event) {
  const fields = Object.entries(event)
    .filter(([, v]) => v !== undefined && v !== '')
    .map(([k, v]) => `${k}:${quote(v)}`);
  return ` {${fields.join(',')}}`;
}

const [atom, ics] = await Promise.all([
  get(`${BASE}/calendar/feed.xml`).then(parseAtom),
  get(`${BASE}/calendar/feeds/all.ics`).then(parseIcs),
]);

const events = atom.map((entry) => {
  const extra = ics.get(entry.id);
  const start = extra?.DTSTART ? icsTime(extra.DTSTART) : entry.start;
  const rawEnd = extra?.DTEND ? icsTime(extra.DTEND) : null;
  // A three-hour event and an event with no end look the same in the .ics, so
  // take the suspicious one as unknown and show only the start.
  const end = rawEnd && rawEnd - start !== INVENTED_END_MS ? rawEnd : null;
  const sources = extra ? sourcesOf(extra.DESCRIPTION ?? '') : [];
  const [source, extraSource] = sources.map(([, url]) => url);
  const label = sources[0]?.[0] ?? '';
  const hub = `${BASE}/calendar#${entry.id}`;

  return {
    id: entry.id,
    title: entry.title,
    category: first(CATEGORIES, entry.group) ?? 'Community',
    group: entry.group,
    day: String(Number(berlin(start, { day: 'numeric' }))),
    weekday: berlin(start, { weekday: 'short' }).toUpperCase(),
    month: berlin(start, { month: 'short' }).toUpperCase(),
    date: isoDay(start),
    start: start.toISOString().replace('.000', ''),
    end: end ? end.toISOString().replace('.000', '') : undefined,
    time: end ? `${clock(start)}–${clock(end)}` : clock(start),
    place: entry.place || 'Location in the announcement',
    summary: entry.summary,
    source: source ?? hub,
    platform: source ? platformOf(source, label) : 'Rationality Munich',
    format: first(FORMATS, entry.title, entry.summary) ?? 'Community event',
    extraSource,
  };
});

if (events.length === 0) throw new Error('snapshot: the feed had no upcoming events');

const today = new Date();
const file = `// Generated by tools/snapshot.mjs from the live calendar. Do not edit by
// hand: run \`pnpm snapshot\` to take a fresh one.
export type CommunityEvent = {id:string; title:string; category:string; group:string; day:string; weekday:string; month:string; date:string; start:string; end?:string; time:string; place:string; summary:string; source:string; platform:string; format:string; extraSource?:string};
/** The day this snapshot was taken. The pages date themselves by it, and the
 * month grids take it for today. */
export const SNAPSHOT = '${isoDay(today)}';
export const SNAPSHOT_LABEL = '${longDay(today)}';
export const events: CommunityEvent[] = [
${events.map(literal).join(',\n')}
];
`;

await fs.writeFile(path.join(root, 'lib/content.ts'), file);
console.log(
  `Snapshot of ${events.length} upcoming events taken on ${longDay(today)}; ` +
    `run \`pnpm assets\` for the .ics downloads.`,
);
