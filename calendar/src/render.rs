//! The HTML pages: upcoming events, past events and statistics.

use chrono::{DateTime, Utc};
use chrono_tz::Europe::Berlin;

use crate::event::{COMBO_GROUPS, DEFAULT_GROUPS, Event, GROUPS, group_name};
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
    if let Some((n, _)) = e.attended {
        meta.push(format!("about {n} came"));
    }
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
    let links = if e.chat {
        r#"<li><span class="chat">In the group chat</span></li>"#.to_string()
    } else {
        links
    };
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
        r##"<article id="{id}" data-groups="{groups}"{hidden}><div class="date"><div class="wd">{wd}</div><div class="d">{day}</div></div><div><div class="head"><h3><a href="#{id}">{title}</a></h3>{add}</div><p class="meta">{meta}</p>{text}<ul class="links">{links}</ul></div></article>"##,
        id = e.id,
        groups = e.groups.join(" "),
        // What the page shows before the visitor picks; the script agrees.
        hidden = if e.in_any(DEFAULT_GROUPS) {
            ""
        } else {
            " hidden"
        },
        wd = start.format("%a"),
        day = start.format("%-d"),
        title = html_escape(&e.title),
    )
}

fn by_month(events: &[&Event], upcoming: bool) -> String {
    let mut months: Vec<(String, Vec<&Event>)> = Vec::new();
    for e in events {
        let this = e.start.with_timezone(&Berlin).format("%B %Y").to_string();
        match months.last_mut() {
            Some((m, list)) if *m == this => list.push(e),
            _ => months.push((this, vec![e])),
        }
    }
    let mut out = String::new();
    for (month, list) in months {
        let hidden = if list.iter().any(|e| e.in_any(DEFAULT_GROUPS)) {
            ""
        } else {
            " hidden"
        };
        out += &format!("<section class=\"month\"{hidden}><h2>{month}</h2>\n");
        for e in list {
            out += &render_event(e, upcoming);
            out += "\n";
        }
        out += "</section>\n";
    }
    out
}

fn filters(events: &[&Event]) -> String {
    let chip = |(key, name): &(&str, &str)| {
        let n = events.iter().filter(|e| e.in_any(&[key])).count();
        let checked = if DEFAULT_GROUPS.contains(key) {
            " checked"
        } else {
            ""
        };
        format!(
            r#"<label><input type="checkbox" value="{key}" data-name="{name}"{checked}> {name} <span class="n">{n}</span></label>"#,
            name = html_escape(name),
        )
    };
    let (main, more): (Vec<_>, Vec<_>) = GROUPS.iter().partition(|(k, _)| COMBO_GROUPS.contains(k));
    let main: String = main.into_iter().map(chip).collect();
    let more: String = more.into_iter().map(chip).collect();
    format!(
        r#"<div class="filters" id="filters" role="group" aria-label="Groups to show" data-combo="{}" hidden><div class="row">{main}</div><div class="row more"><span class="label">More groups:</span>{more}</div></div>"#,
        COMBO_GROUPS.join(" ")
    )
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

fn subscribe() -> String {
    let feed = format!(
        "rationality-munich.com/calendar/feeds/{}.ics",
        DEFAULT_GROUPS.join("+")
    );
    // The script rewrites this for the groups picked; see `subscribe` in page.html.
    format!(
        r#"<p class="subscribe" id="subscribe"><a href="webcal://{feed}">Subscribe in your calendar app</a>, or add <code>https://{feed}</code> by URL.</p>"#
    )
}

pub fn page(p: &Page) -> String {
    let past = r#"<a href="/calendar/past/" data-keep-filter>Past events</a>"#;
    let upcoming = r#"<a href="/calendar" data-keep-filter>Upcoming</a>"#;
    let stats = r#"<a href="/calendar/stats/" data-keep-filter>Statistics</a>"#;
    let (title, heading, lede, nav, key) = match p.kind {
        Kind::Upcoming => (
            "Calendar · Rationality Munich",
            "Upcoming events",
            "Events from the rationality and EA groups in Munich. More groups are one click away.",
            format!("{past} · {stats}"),
            "upcoming",
        ),
        Kind::Past => (
            "Past events · Rationality Munich",
            "Past events",
            "Newest first.",
            format!("{upcoming} · {stats}"),
            "past",
        ),
        Kind::Stats => (
            "Statistics · Rationality Munich",
            "Statistics",
            "How often the groups meet.",
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
        .replace(
            "{{canonical}}",
            &match p.kind {
                Kind::Upcoming => BASE.to_string(),
                Kind::Past => format!("{BASE}/past/"),
                Kind::Stats => format!("{BASE}/stats/"),
            },
        )
        .replace("{{filters}}", &filters(&p.events))
        .replace(
            "{{subscribe}}",
            &if listing { subscribe() } else { String::new() },
        )
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
    fn hidden_beats_the_display_rules() {
        // Articles are `display: grid`, which would otherwise override the
        // browser's own `[hidden] { display: none }`.
        assert!(PAGE.contains("[hidden] { display: none !important; }"));
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
