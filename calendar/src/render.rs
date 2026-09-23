//! The HTML pages: upcoming events, past events and statistics.

use chrono::{DateTime, Utc};
use chrono_tz::Europe::Berlin;

use crate::event::{Event, GROUPS, group_name};
use crate::ics;
use crate::stats;

pub const BASE: &str = "https://rationality-munich.com/calendar";
const PAGE: &str = include_str!("page.html");

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn when(e: &Event) -> String {
    let start = e.start.with_timezone(&Berlin);
    let mut s = start.format("%H:%M").to_string();
    if let Some(end) = e.end.map(|t| t.with_timezone(&Berlin)) {
        if end.date_naive() == start.date_naive() {
            s += &end.format("–%H:%M").to_string();
        } else {
            s += &end.format(" – %a %-d %b %H:%M").to_string();
        }
    }
    s
}

/// Opens Google Calendar's "new event" form, filled in. Nothing is sent to
/// Google unless the visitor clicks it.
fn google_link(e: &Event) -> String {
    format!(
        "https://calendar.google.com/calendar/render?action=TEMPLATE&text={}&dates={}/{}&details={}&location={}",
        percent_encode(&e.title),
        ics::utc(e.start),
        ics::utc(e.end_or_default()),
        percent_encode(&ics::description(e, &format!("{BASE}#{}", e.id))),
        percent_encode(e.place()),
    )
}

/// A calendar with a plus: download this event as an .ics file.
const ICON_ADD: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4M16 3v4M12 13v5M9.5 15.5h5"/></svg>"#;

/// A calendar with a G, rather than Google's own logo.
const ICON_GOOGLE: &str = r#"<svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4M16 3v4"/><text x="12" y="19" text-anchor="middle" font-size="8.5" font-weight="700" font-family="system-ui, sans-serif" fill="currentColor" stroke="none">G</text></svg>"#;

fn render_event(e: &Event, upcoming: bool) -> String {
    let start = e.start.with_timezone(&Berlin);
    let mut meta = vec![when(e)];
    if !e.place().is_empty() {
        meta.push(e.place().to_string());
    }
    meta.push(
        e.groups
            .iter()
            .map(|g| group_name(g))
            .collect::<Vec<_>>()
            .join(" & "),
    );
    let meta = meta
        .iter()
        .map(|m| html_escape(m))
        .collect::<Vec<_>>()
        .join(" · ");
    let links: String = e
        .links
        .iter()
        .map(|(label, url)| {
            format!(
                r#"<li><a href="{}">{} ↗</a></li>"#,
                html_escape(url),
                html_escape(label)
            )
        })
        .collect();
    let add = if upcoming {
        format!(
            r#"<div class="add"><a href="/calendar/e/{id}.ics" download="{id}.ics" title="Add to your calendar (.ics)" aria-label="Add to your calendar">{ICON_ADD}</a><a href="{g}" title="Add to Google Calendar" aria-label="Add to Google Calendar">{ICON_GOOGLE}</a></div>"#,
            id = e.id,
            g = html_escape(&google_link(e)),
        )
    } else {
        String::new()
    };
    let text = if e.excerpt.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="text">{}</p>"#, html_escape(&e.excerpt))
    };
    format!(
        r##"<article id="{id}" data-groups="{groups}"><div class="date"><div class="wd">{wd}</div><div class="d">{day}</div></div><div><div class="head"><h3><a href="#{id}">{title}</a></h3>{add}</div><p class="meta">{meta}</p>{text}<ul class="links">{links}</ul></div></article>"##,
        id = e.id,
        groups = e.groups.join(" "),
        wd = start.format("%a"),
        day = start.format("%-d"),
        title = html_escape(&e.title),
    )
}

fn by_month(events: &[&Event], upcoming: bool) -> String {
    let mut out = String::new();
    let mut month = None;
    for e in events {
        let this = e.start.with_timezone(&Berlin).format("%B %Y").to_string();
        if month.as_ref() != Some(&this) {
            if month.is_some() {
                out += "</section>\n";
            }
            out += &format!("<section class=\"month\"><h2>{this}</h2>\n");
            month = Some(this);
        }
        out += &render_event(e, upcoming);
        out += "\n";
    }
    if month.is_some() {
        out += "</section>\n";
    }
    out
}

fn filters(events: &[&Event]) -> String {
    let boxes: String = GROUPS
        .iter()
        .map(|(key, name)| {
            let n = events.iter().filter(|e| e.in_any(&[key])).count();
            format!(
                r#"<label><input type="checkbox" value="{key}" checked> {name} <span class="n">{n}</span></label>"#
            )
        })
        .collect();
    format!(r#"<form class="filters" id="filters" hidden>{boxes}</form>"#)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Upcoming,
    Past,
    Stats,
}

pub struct Page<'a> {
    pub kind: Kind,
    pub events: Vec<&'a Event>,
    pub stale: &'a [String],
    pub now: DateTime<Utc>,
}

const SUBSCRIBE: &str = r#"<p class="subscribe"><a id="sub" href="webcal://rationality-munich.com/calendar/feeds/all.ics">Subscribe in your calendar app</a> to get these events there, kept up to date. Or add <code id="suburl">https://rationality-munich.com/calendar/feeds/all.ics</code> by URL.</p>"#;

pub fn page(p: &Page) -> String {
    let past = r#"<a href="/calendar/past/" data-keep-filter>Past events</a>"#;
    let upcoming = r#"<a href="/calendar" data-keep-filter>Upcoming</a>"#;
    let stats = r#"<a href="/calendar/stats/" data-keep-filter>Statistics</a>"#;
    let (title, heading, lede, nav, key) = match p.kind {
        Kind::Upcoming => (
            "Calendar · Rationality Munich",
            "Upcoming events",
            "Events from the rationality, EA and philosophy groups in Munich, collected from LessWrong, the EA Forum, Meetup, Luma and Philosophia. Regular events are listed on the <a href=\"/#regular\">home page</a>.",
            format!("{past} · {stats}"),
            "upcoming",
        ),
        Kind::Past => (
            "Past events · Rationality Munich",
            "Past events",
            "Everything these groups have held that we know of, newest first.",
            format!("{upcoming} · {stats}"),
            "past",
        ),
        Kind::Stats => (
            "Statistics · Rationality Munich",
            "Statistics",
            "How often the groups meet, from every event we know of.",
            format!("{upcoming} · {past}"),
            "stats",
        ),
    };
    let body = match p.kind {
        Kind::Stats => stats::render(&p.events),
        _ if p.events.is_empty() => format!(
            r#"<p class="empty">{}</p>"#,
            if p.kind == Kind::Upcoming {
                "No upcoming events announced right now."
            } else {
                "Nothing here yet."
            }
        ),
        _ => by_month(&p.events, p.kind == Kind::Upcoming),
    };
    let listing = p.kind != Kind::Stats;
    let note = if p.stale.is_empty() {
        String::new()
    } else {
        format!(
            " Couldn't reach {} this time, so their events may be out of date.",
            p.stale.join(", ")
        )
    };
    PAGE.replace("{{title}}", title)
        .replace("{{heading}}", heading)
        .replace("{{lede}}", lede)
        .replace("{{nav}}", &format!("<span>{nav}</span>"))
        .replace("{{page}}", key)
        .replace("{{filters}}", &filters(&p.events))
        .replace("{{subscribe}}", if listing { SUBSCRIBE } else { "" })
        .replace("{{body}}", &body)
        .replace(
            "{{updated}}",
            &p.now
                .with_timezone(&Berlin)
                .format("%-d %b %Y, %H:%M")
                .to_string(),
        )
        .replace("{{stale}}", &html_escape(&note))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{assign_ids, sanitize, tests::ev};

    #[test]
    fn events_carry_their_permalink_groups_and_calendar_links() {
        let mut events = vec![ev("Petrov Day", 0, "LessWrong")];
        assign_ids(&mut events);
        let html = page(&Page {
            kind: Kind::Upcoming,
            events: events.iter().collect(),
            stale: &[],
            now: Utc::now(),
        });
        assert!(html.contains(r#"<article id="2026-09-26-petrov-day" data-groups="ea">"#));
        assert!(html.contains(r##"href="#2026-09-26-petrov-day""##));
        assert!(html.contains(r#"href="/calendar/e/2026-09-26-petrov-day.ics""#));
        assert!(
            html.contains(
                "calendar.google.com/calendar/render?action=TEMPLATE&amp;text=Petrov%20Day"
            )
        );
    }

    #[test]
    fn nothing_from_a_feed_reaches_the_page_unescaped() {
        let mut e = ev("<script>alert(1)</script>", 0, "A");
        e.location = r#""><img src=x>"#.into();
        let mut events = sanitize(vec![e]);
        assign_ids(&mut events);
        let html = page(&Page {
            kind: Kind::Upcoming,
            events: events.iter().collect(),
            stale: &[],
            now: Utc::now(),
        });
        assert!(!html.contains("<script>alert") && !html.contains("<img src=x>"));
    }
}
