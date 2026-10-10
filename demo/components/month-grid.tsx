import Link from 'next/link';
import { events as allEvents, SNAPSHOT, type CommunityEvent } from '@/lib/content';

/** Monday first, the way a wall calendar runs here. */
const WEEKDAYS = [
  ['Mon', 'Monday'],
  ['Tue', 'Tuesday'],
  ['Wed', 'Wednesday'],
  ['Thu', 'Thursday'],
  ['Fri', 'Friday'],
  ['Sat', 'Saturday'],
  ['Sun', 'Sunday'],
] as const;

export const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
] as const;

/** Dates are plain YYYY-MM-DD, so the arithmetic stays in UTC and out of trouble. */
const iso = (d: Date) => d.toISOString().slice(0, 10);
const stamp = (year: number, month: number) => `${year}-${String(month + 1).padStart(2, '0')}`;

/**
 * The weeks a month needs as rows of seven, padded at both ends with the
 * neighbouring month's days so every row is a full week.
 */
function weeksOf(year: number, month: number) {
  const lead = (new Date(Date.UTC(year, month, 1)).getUTCDay() + 6) % 7;
  const length = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
  const rows = Math.ceil((lead + length) / 7);
  return Array.from({ length: rows }, (_, w) =>
    Array.from({ length: 7 }, (_, d) => new Date(Date.UTC(year, month, 1 - lead + w * 7 + d))),
  );
}

/**
 * The window both pages show: the month we are in and the one after it. A
 * static export cannot know what day it is when someone visits, so "now" is
 * the day `pnpm snapshot` last ran — which `pnpm publish-demo` does every time.
 */
export function currentAndNext(from: string = SNAPSHOT) {
  const year = Number(from.slice(0, 4));
  const month = Number(from.slice(5, 7)) - 1;
  return [
    { year, month },
    { year: month === 11 ? year + 1 : year, month: (month + 1) % 12 },
  ];
}

/** How many of these events fall in a given month. */
export function countIn(events: CommunityEvent[], year: number, month: number) {
  return events.filter((e) => e.date.startsWith(stamp(year, month))).length;
}

function monthName({ year, month }: { year: number; month: number }) {
  return `${MONTHS[month]} ${year}`;
}

/**
 * One month as a seven-column week grid. A table rather than a CSS grid: it
 * is a table, and the weekday headers then mean something read aloud.
 */
export function MonthGrid({
  year,
  month,
  events = allEvents,
}: {
  year: number;
  month: number;
  events?: CommunityEvent[];
}) {
  const label = monthName({ year, month });
  const byDay: Record<string, CommunityEvent[]> = {};
  for (const event of events.filter((e) => e.date.startsWith(stamp(year, month)))) {
    (byDay[event.date] ??= []).push(event);
  }

  return (
    <section className="month-scroll" tabIndex={0} aria-label={`${label}, as a calendar`}>
      <table className="month-grid">
        <caption className="sr-only">{label}, one row per week</caption>
        <thead>
          <tr>
            {WEEKDAYS.map(([short, long]) => (
              <th scope="col" key={long}>
                <abbr title={long}>{short}</abbr>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {weeksOf(year, month).map((week) => (
            <tr key={iso(week[0])}>
              {week.map((day) => {
                const date = iso(day);
                const inMonth = day.getUTCMonth() === month;
                // The neighbouring month's days keep the rows square but stay
                // empty: its own grid is where its events belong.
                const dayEvents = inMonth ? (byDay[date] ?? []) : [];
                const classes = ['day'];
                if (!inMonth) classes.push('outside');
                if (dayEvents.length > 0) classes.push('busy');
                if (date === SNAPSHOT) classes.push('snapshot');
                return (
                  <td key={date} className={classes.join(' ')}>
                    <span className="day-number">
                      {day.getUTCDate()}
                      {date === SNAPSHOT && <span className="sr-only"> — snapshot taken</span>}
                    </span>
                    {dayEvents.map((event) => (
                      <Link key={event.id} className="day-event" href={`/events/${event.id}`}>
                        <b>{event.time}</b>
                        {event.title}
                      </Link>
                    ))}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}
