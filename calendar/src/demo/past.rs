//! Every past event, newest first, and the form for reporting how many
//! came.
//!
//! The report posts to the attendance service at `/calendar/attend`, which
//! sends the browser back to the event it was reported for, so the page
//! needs no script. Only events from the last few months carry the form:
//! nobody is going to report a headcount for 2019, and 1500 forms would
//! make the page several times heavier than it already is.

use chrono::{DateTime, Datelike, Utc};
use chrono_tz::Europe::Berlin;

use super::{clock, group_list, groups, place, plural, turnout};
use crate::event::Event;
use crate::render::html_escape;

/// How long after an event a headcount can still be reported.
const REPORT_DAYS: i64 = 120;

/// One past event. These have no page of their own, so the title is plain
/// text and the links go to the announcements.
fn row(e: &Event, reportable: bool) -> String {
    let start = e.start.with_timezone(&Berlin);
    let text = if e.excerpt.is_empty() {
        String::new()
    } else {
        format!("<p>{}</p>", html_escape(&e.excerpt))
    };
    let links: String = if e.chat {
        r#"<span class="tag">In the group chat</span>"#.to_string()
    } else {
        e.links
            .iter()
            .map(|(label, url)| {
                format!(
                    r#"<a class="text-link" href="{url}">{label} ↗</a>"#,
                    url = html_escape(url),
                    label = html_escape(label),
                )
            })
            .collect()
    };
    let coming = match turnout(e) {
        (head, _) if head.is_empty() => String::new(),
        (head, Some(note)) => format!("<span>{head}, {}</span>", note.to_lowercase()),
        (head, None) => format!("<span>{head}</span>"),
    };
    let report = if reportable {
        format!(
            r#"<form class="report" method="post" action="/calendar/attend"><input type="hidden" name="id" value="{id}"><label for="n-{id}">How many came?</label><input id="n-{id}" name="n" type="number" min="1" max="1000" inputmode="numeric" required><button type="submit">Report</button></form><p class="said ok" id="r-{id}">Counted, thank you. It shows on this page within the hour.</p><p class="said no" id="x-{id}">Not counted. This browser may have reported it already.</p>"#,
            id = e.id,
        )
    } else {
        String::new()
    };
    format!(
        r#"<article id="{id}" class="calendar-row past-row"><time datetime="{iso}" class="calendar-date"><span>{wd}</span><strong>{d}</strong><span>{mon}</span></time><div class="calendar-row-content"><div class="calendar-row-labels"><span class="tag">{category}</span><span class="event-group">{groups}</span></div><h3>{title}</h3><div class="event-meta"><span>{time}</span><span>{place}</span>{coming}</div>{text}<div class="past-links">{links}</div>{report}</div></article>"#,
        id = e.id,
        iso = start.format("%Y-%m-%d"),
        wd = start.format("%a").to_string().to_uppercase(),
        d = start.format("%-d"),
        mon = start.format("%b").to_string().to_uppercase(),
        category = html_escape(groups::category(e)),
        groups = html_escape(&group_list(e)),
        title = html_escape(&e.title),
        time = html_escape(&clock(e)),
        place = html_escape(place(e)),
    )
}

/// Every past event, newest first. `events` arrives oldest first.
pub fn page(events: &[&Event], now: DateTime<Utc>) -> String {
    let cutoff = now - chrono::Duration::days(REPORT_DAYS);
    let newest_first: Vec<&&Event> = events.iter().rev().collect();

    let years: Vec<i32> = {
        let mut seen: Vec<i32> = newest_first
            .iter()
            .map(|e| e.start.with_timezone(&Berlin).year())
            .collect();
        seen.dedup();
        seen
    };
    let nav: String = years
        .iter()
        .map(|y| format!(r##"<a class="tag" href="#y{y}">{y}</a>"##))
        .collect();

    let mut sections = String::new();
    let mut month: Option<String> = None;
    let mut year: Option<i32> = None;
    let mut rows = String::new();
    let mut count = 0usize;
    let mut open = false;

    let flush = |rows: &mut String, month: &Option<String>, count: &mut usize, out: &mut String| {
        if let Some(label) = month {
            out.push_str(&format!(
                r#"<section class="calendar-month"><div class="month-heading"><h2>{label}</h2><span>{n} event{s}</span></div>{rows}</section>"#,
                n = count,
                s = plural(*count),
            ));
        }
        rows.clear();
        *count = 0;
    };

    for e in &newest_first {
        let local = e.start.with_timezone(&Berlin);
        let this_month = local.format("%B %Y").to_string();
        if month.as_deref() != Some(this_month.as_str()) {
            flush(&mut rows, &month, &mut count, &mut sections);
            if year != Some(local.year()) {
                if open {
                    sections.push_str("</div>");
                }
                sections.push_str(&format!(
                    r#"<div class="past-year" id="y{y}"><h2 class="year-heading">{y}</h2>"#,
                    y = local.year()
                ));
                year = Some(local.year());
                open = true;
            }
            month = Some(this_month);
        }
        rows.push_str(&row(e, e.start >= cutoff));
        count += 1;
    }
    flush(&mut rows, &month, &mut count, &mut sections);
    if open {
        sections.push_str("</div>");
    }

    format!(
        r##"<main id="main" class="wrap calendar-page">
<div class="page-heading"><p class="eyebrow">PAST EVENTS</p><div class="title-actions"><h1>Everything that has happened</h1></div><p>{n} event{s} the Munich groups have held, newest first. Several of them were organised only in a group chat and were added by hand.</p>
<nav class="year-nav" aria-label="Jump to a year">{nav}</nav></div>
<div class="calendar-context"><span><span class="status-dot"></span> {n} event{s}</span><span>All times Munich local time</span></div>
{sections}
<aside class="coverage-note"><h3>How many came</h3><p>Events from the last {days} days take a headcount from anyone who was there. The number shown is the middle of what people reported, unless an organiser gave a count. Nothing about you is stored with it.</p><div class="actions"><a class="text-link" href="/calendar/">Upcoming events</a><a class="text-link" href="/calendar/stats/">Statistics</a></div></aside>
</main>"##,
        n = events.len(),
        s = plural(events.len()),
        days = REPORT_DAYS,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::tests::ev;

    fn at(days: i64) -> DateTime<Utc> {
        Utc::now() - chrono::Duration::days(days)
    }

    #[test]
    fn events_are_grouped_by_month_newest_first() {
        let mut old = ev("Older dinner", 0, "A");
        old.start = at(400);
        let mut recent = ev("Recent talk", 0, "B");
        recent.start = at(10);
        let html = page(&[&old, &recent], Utc::now());
        let first = html.find("Recent talk").expect("the recent one is there");
        let second = html.find("Older dinner").expect("the old one is there");
        assert!(first < second, "newest first");
        assert!(html.contains("2 events"));
    }

    #[test]
    fn only_recent_events_take_a_headcount() {
        let mut old = ev("Older dinner", 0, "A");
        old.start = at(400);
        let mut recent = ev("Recent talk", 0, "B");
        recent.start = at(10);
        let html = page(&[&old, &recent], Utc::now());
        let forms = html.matches(r#"action="/calendar/attend""#).count();
        assert_eq!(forms, 1, "only the recent event takes a report");
        assert!(html.contains(&format!(r#"value="{}""#, recent.id)));
    }

    #[test]
    fn the_form_needs_no_script() {
        let mut recent = ev("Recent talk", 0, "B");
        recent.start = at(3);
        let html = page(&[&recent], Utc::now());
        assert!(!html.contains("<script"), "the page runs no script");
        assert!(html.contains(r#"method="post""#));
        // Where the attendance service sends the browser back to.
        assert!(html.contains(&format!(r#"id="r-{}""#, recent.id)));
        assert!(html.contains(&format!(r#"id="x-{}""#, recent.id)));
    }
}
