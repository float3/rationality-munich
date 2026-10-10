'use client';
import { useEffect, useState } from 'react';
import Link from 'next/link';
import { ArrowUpRight, Clock, MapPin, CalendarDays, RefreshCw } from 'lucide-react';
import { events } from '@/lib/content';
import { MONTHS, MonthGrid, countIn, currentAndNext } from '@/components/month-grid';

export function CalendarList() {
  const [mode, setMode] = useState('snapshot');
  useEffect(() => {
    const m = new URLSearchParams(window.location.search).get('preview');
    if (m === 'empty' || m === 'unavailable') setMode(m);
  }, []);

  if (mode === 'empty') {
    return (
      <div className="empty-state">
        <CalendarDays size={36} />
        <h2>Nothing listed.</h2>
        <p>
          No upcoming events are listed in this example state. Get event invitations or check with
          the groups directly.
        </p>
        <div className="actions">
          <Link className="button primary" href="/subscribe">
            Get event invitations
          </Link>
          <Link className="text-link" href="/#community">
            Explore the groups ↗
          </Link>
        </div>
        <button className="text-link" onClick={() => setMode('snapshot')}>
          Return to the event snapshot ↩
        </button>
      </div>
    );
  }

  return (
    <>
      {mode === 'unavailable' && (
        <div className="notice warning" role="status">
          <RefreshCw size={20} />
          <div>
            <strong>Updates are temporarily unavailable.</strong>
            <p>
              This is an example of a feed outage. Last-known events remain below; confirm details
              with the original organizers.
            </p>
            <button className="text-link" onClick={() => setMode('snapshot')}>
              Return to the event snapshot ↩
            </button>
          </div>
        </div>
      )}
      {currentAndNext().map(({ year, month }) => {
        const n = countIn(events, year, month);
        return (
          <section key={`${year}-${month}`} className="calendar-month">
            <div className="month-heading">
              <h2>
                {MONTHS[month]} <span>{year}</span>
              </h2>
              <span>
                {n} event{n === 1 ? '' : 's'}
              </span>
            </div>
            <MonthGrid year={year} month={month} />
          </section>
        );
      })}
      {/* The grids stop at next month; anything further off is still listed. */}
      <section className="calendar-month">
        <div className="month-heading">
          <h2>Every upcoming event</h2>
          <span>
            {events.length} event{events.length === 1 ? '' : 's'}
          </span>
        </div>
        {events.map((event) => (
          <article key={event.id} className="calendar-row">
            <time dateTime={event.date} className="calendar-date">
              <span>{event.weekday}</span>
              <strong>{event.day}</strong>
              <span>{event.month}</span>
            </time>
            <div className="calendar-row-content">
              <div className="calendar-row-labels">
                <span className="tag">{event.category}</span>
                <span className="event-group">{event.group}</span>
              </div>
              <h3>
                <Link href={`/events/${event.id}`}>{event.title}</Link>
              </h3>
              <div className="event-meta">
                <span>
                  <Clock size={15} />
                  {event.time}
                </span>
                <span>
                  <MapPin size={15} />
                  {event.place}
                </span>
              </div>
              <p>{event.summary}</p>
            </div>
            <Link className="text-link calendar-row-action" href={`/events/${event.id}`}>
              Event details <ArrowUpRight size={18} />
            </Link>
          </article>
        ))}
      </section>
    </>
  );
}
