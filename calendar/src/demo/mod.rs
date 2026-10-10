//! The site itself: the hub, the calendar and a page per event, from the
//! same merged `Event`s as /calendar. `AT` is the only place a URL prefix is
//! written down, so these pages can also be served under one.
//!
//! Written by the same hourly run, from the same merged `Event`s, so there is
//! nothing to refresh by hand. Plain HTML with one stylesheet and no script.
//!
//! Output, under `$STATE_DIRECTORY/demo/`, served at the root:
//!
//! - `index.html`: the hub — what is on, the groups, what is being planned
//! - `calendar/index.html`: this month and next as week grids, then the list
//! - `events/<id>/index.html`: one page per upcoming event
//! - `tools/`: what this site hosts for other groups to use
//! - `subscribe/`, `privacy/`, `impressum/`, `about/`: the pages of words
//! - `404.html`, `robots.txt`, `sitemap.xml`, `.well-known/security.txt`
//! - `style.css`, `og.png`, `munich.webp`, `favicon.svg`

mod groups;
mod plans;
mod tools;

use std::fs;
use std::path::Path;

use chrono::{DateTime, Datelike, Days, Duration, NaiveDate, Utc};
use chrono_tz::Europe::Berlin;

use crate::event::{DEFAULT_GROUPS, Event, group_name};
use crate::render::html_escape;

/// Where this is served. Empty: these are the site's own pages, at the
/// root. Every link, canonical URL and asset path is built from it, and a
/// test checks nothing writes a prefix out by hand.
pub const AT: &str = "";
const SITE: &str = "https://rationality-munich.com";

const PAGE: &str = include_str!("page.html");
const CSS: &str = include_str!("style.css");
const FAVICON: &str = include_str!("favicon.svg");
const PHOTO: &[u8] = include_bytes!("munich.webp");
const OG: &[u8] = include_bytes!("og.png");

const ARROW: &str = r#"<span class="arrow" aria-hidden="true">↗</span>"#;

/// Monday first, the way a wall calendar runs here.
const WEEKDAYS: [(&str, &str); 7] = [
    ("Mon", "Monday"),
    ("Tue", "Tuesday"),
    ("Wed", "Wednesday"),
    ("Thu", "Thursday"),
    ("Fri", "Friday"),
    ("Sat", "Saturday"),
    ("Sun", "Sunday"),
];

// ------------------------------------------------------------------ bits

/// The start, and the end when the announcement gave one: "18:00–21:00".
fn clock(e: &Event) -> String {
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

fn day(e: &Event) -> NaiveDate {
    e.start.with_timezone(&Berlin).date_naive()
}

fn group_list(e: &Event) -> String {
    e.groups
        .iter()
        .map(|g| group_name(g))
        .collect::<Vec<_>>()
        .join(" & ")
}

/// Where it is. An announcement that names no venue gets a stand-in rather
/// than an empty line.
fn place(e: &Event) -> &str {
    match e.place() {
        "" => "Location in the announcement",
        p => p,
    }
}

/// The organiser's own announcement.
fn source(e: &Event) -> Option<&(String, String)> {
    e.links.first()
}

// ------------------------------------------------------------- month grid

/// The first of the month after this one.
fn next_month(year: i32, month: u32) -> NaiveDate {
    match month {
        12 => NaiveDate::from_ymd_opt(year + 1, 1, 1),
        _ => NaiveDate::from_ymd_opt(year, month + 1, 1),
    }
    .expect("a first of the month is a real date")
}

/// The weeks of a month as rows of seven, padded at both ends with the
/// neighbouring month's days so every row is a full week.
fn weeks_of(year: i32, month: u32) -> Vec<Vec<NaiveDate>> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("a first of the month is real");
    let lead = u64::from(first.weekday().num_days_from_monday());
    let start = first - Days::new(lead);
    let length = (next_month(year, month) - first).num_days() as u64;
    let rows = (lead + length).div_ceil(7);
    (0..rows)
        .map(|w| (0..7).map(|d| start + Days::new(w * 7 + d)).collect())
        .collect()
}

/// One month as a seven-column week grid. A table rather than a CSS grid, so
/// the weekday headers are announced as column headers.
fn month_grid(year: i32, month: u32, events: &[&Event], today: NaiveDate) -> String {
    let label = NaiveDate::from_ymd_opt(year, month, 1)
        .expect("a first of the month is real")
        .format("%B %Y")
        .to_string();
    let head: String = WEEKDAYS
        .iter()
        .map(|(short, long)| format!(r#"<th scope="col"><abbr title="{long}">{short}</abbr></th>"#))
        .collect();
    let mut rows = String::new();
    for week in weeks_of(year, month) {
        rows += "<tr>";
        for date in week {
            // The neighbouring month's days fill out the row but stay empty;
            // their events belong in that month's own grid.
            let inside = date.month() == month && date.year() == year;
            let today_here: Vec<&&Event> = if inside {
                events.iter().filter(|e| day(e) == date).collect()
            } else {
                Vec::new()
            };
            let mut classes = vec!["day"];
            if !inside {
                classes.push("outside");
            }
            if !today_here.is_empty() {
                classes.push("busy");
            }
            let marker = if date == today {
                classes.push("today");
                r#"<span class="sr-only"> — today</span>"#
            } else {
                ""
            };
            let cards: String = today_here
                .iter()
                .map(|e| {
                    format!(
                        r#"<a class="day-event" href="{AT}/events/{id}/"><b>{time}</b>{title}</a>"#,
                        id = e.id,
                        time = html_escape(&clock(e)),
                        title = html_escape(&e.title),
                    )
                })
                .collect();
            rows += &format!(
                r#"<td class="{classes}"><span class="day-number">{n}{marker}</span>{cards}</td>"#,
                classes = classes.join(" "),
                n = date.day(),
            );
        }
        rows += "</tr>";
    }
    format!(
        r#"<section class="month-scroll" tabindex="0" aria-label="{label}, as a calendar"><table class="month-grid"><caption class="sr-only">{label}, one row per week</caption><thead><tr>{head}</tr></thead><tbody>{rows}</tbody></table></section>"#
    )
}

/// The month we are in and the one after it: the window both pages show.
fn current_and_next(today: NaiveDate) -> [(i32, u32); 2] {
    let after = next_month(today.year(), today.month());
    [(today.year(), today.month()), (after.year(), after.month())]
}

fn month_count(events: &[&Event], year: i32, month: u32) -> usize {
    events
        .iter()
        .filter(|e| {
            let d = day(e);
            d.year() == year && d.month() == month
        })
        .count()
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

// ------------------------------------------------------------- event rows

fn event_row(e: &Event) -> String {
    let start = e.start.with_timezone(&Berlin);
    let text = if e.excerpt.is_empty() {
        String::new()
    } else {
        format!("<p>{}</p>", html_escape(&e.excerpt))
    };
    format!(
        r#"<article id="{id}" class="calendar-row"><time datetime="{iso}" class="calendar-date"><span>{wd}</span><strong>{d}</strong><span>{mon}</span></time><div class="calendar-row-content"><div class="calendar-row-labels"><span class="tag">{category}</span><span class="event-group">{groups}</span></div><h3><a href="{AT}/events/{id}/">{title}</a></h3><div class="event-meta"><span>{time}</span><span>{place}</span></div>{text}</div><a class="text-link calendar-row-action" href="{AT}/events/{id}/">Event details {ARROW}</a></article>"#,
        iso = start.format("%Y-%m-%d"),
        wd = start.format("%a").to_string().to_uppercase(),
        d = start.format("%-d"),
        mon = start.format("%b").to_string().to_uppercase(),
        category = html_escape(groups::category(e)),
        groups = html_escape(&group_list(e)),
        id = e.id,
        title = html_escape(&e.title),
        time = html_escape(&clock(e)),
        place = html_escape(place(e)),
    )
}

// ------------------------------------------------------------------ pages

struct Shell {
    title: String,
    description: String,
    canonical: String,
    body: String,
}

fn shell(s: Shell, now: DateTime<Utc>, stale: &[String]) -> String {
    let note = if stale.is_empty() {
        String::new()
    } else {
        format!(
            " Couldn’t reach {} this time, so their events may be out of date.",
            stale.join(", ")
        )
    };
    PAGE.replace("{{at}}", AT)
        .replace("{{title}}", &html_escape(&s.title))
        .replace("{{description}}", &html_escape(&s.description))
        .replace("{{canonical}}", &s.canonical)
        .replace("{{body}}", &s.body)
        .replace("{{stale}}", &html_escape(&note))
        .replace(
            "{{updated}}",
            &now.with_timezone(&Berlin)
                .format("%-d %b %Y, %H:%M")
                .to_string(),
        )
}

fn home(events: &[&Event], today: NaiveDate) -> String {
    // Empty when there is nothing on, rather than "0 events coming up".
    let count = match events.len() {
        0 => String::new(),
        n => format!(
            r#"<p class="eyebrow"><span class="status-dot"></span> {n} EVENT{plural} COMING UP</p>"#,
            plural = if n == 1 { "" } else { "S" },
        ),
    };
    let next = events.first();
    let hero_card = match next {
        Some(e) => format!(
            r#"<a class="hero-event" href="{AT}/events/{id}/"><span class="eyebrow">NEXT UP · {when}</span><strong>{title}</strong><span>{time} · {place} {ARROW}</span></a>"#,
            id = e.id,
            when = e
                .start
                .with_timezone(&Berlin)
                .format("%A %-d %b")
                .to_string()
                .to_uppercase(),
            title = html_escape(&e.title),
            time = html_escape(&clock(e)),
            place = html_escape(place(e)),
        ),
        None => String::new(),
    };
    let grids: String = current_and_next(today)
        .iter()
        .map(|&(year, month)| {
            format!(
                r#"<div><h3 class="month-label">{label}</h3>{grid}</div>"#,
                label = NaiveDate::from_ymd_opt(year, month, 1)
                    .expect("a first of the month is real")
                    .format("%B <span>%Y</span>"),
                grid = month_grid(year, month, events, today),
            )
        })
        .collect();
    // The first event whose own announcement says newcomers are welcome.
    let welcoming = events.iter().find(|e| {
        let text = e.excerpt.to_lowercase();
        [
            "newcomer",
            "no reading",
            "no prior",
            "no special",
            "no preparation",
            "beginner",
        ]
        .iter()
        .any(|w| text.contains(w))
    });
    let spotlight = match welcoming {
        Some(e) => format!(
            r#"<div class="wrap"><div class="newcomer-spotlight"><div><p class="eyebrow">NEWCOMERS WELCOME</p><h3>{title} · {date}</h3><p>{text}</p></div><a class="text-link" href="{AT}/events/{id}/">Event details {ARROW}</a></div></div>"#,
            title = html_escape(&e.title),
            date = e.start.with_timezone(&Berlin).format("%-d %B"),
            text = html_escape(&e.excerpt),
            id = e.id,
        ),
        None => String::new(),
    };
    let empty = if events.is_empty() {
        r#"<p class="section-note">Nothing is announced for the next two months yet. The groups post as plans firm up.</p>"#
    } else {
        ""
    };

    format!(
        r##"<main id="main">
<section class="hero wrap">
<div class="hero-copy">{count}
<h1>Meetups in Munich for rationality, EA and AI safety</h1>
<p class="hero-intro">Six independent groups run discussions, dinners, workshops and talks in Munich. This page collects them and their events in one place.</p>
<div class="actions"><a class="button primary" href="{AT}/calendar/">See the calendar {ARROW}</a><a class="text-link" href="{AT}/#community">The groups →</a></div>
</div>
<figure class="hero-image"><img src="{AT}/munich.webp" alt="The Monopteros and green lawns in Munich’s Englischer Garten" width="1600" height="979" fetchpriority="high">{hero_card}<figcaption>Photograph: the Englischer Garten, Munich</figcaption></figure>
</section>
<div class="topic-strip"><div class="wrap"><span>RATIONALITY</span><i>✳</i><span>EFFECTIVE ALTRUISM</span><i>✳</i><span>PHILOSOPHY</span><i>✳</i><span>AI SAFETY</span><i>✳</i><span>AI POLICY</span></div></div>
<section class="section wrap" id="events"><div class="section-heading"><div><p class="eyebrow">THIS MONTH AND NEXT</p><h2>Coming up in Munich</h2></div><a href="{AT}/calendar/" class="text-link">All events {ARROW}</a></div><div class="home-months">{grids}</div>{empty}<p class="section-note">All times Munich time. Check the organiser’s announcement before you go.</p></section>
{spotlight}
{plans}
{groups}
{more}
<section class="section wrap" id="tools"><div class="section-heading"><div><p class="eyebrow">TOOLS</p><h2>Things this site hosts</h2></div><a class="text-link" href="{AT}/tools/">All three {ARROW}</a></div><p class="section-note">AnkiQuest, and the Petrov and Arkhipov Day ceremonies. All three are open source and free to use.</p></section>
<section class="wrap newsletter-section"><div class="newsletter-icon" aria-hidden="true">✉</div><div><p class="eyebrow">MAILING LIST</p><h2>Event invites by email</h2><p>Pick the topics you want to hear about. No open or click tracking, and you can unsubscribe or delete your data from any email.</p><a class="button primary" href="{AT}/subscribe/">Get event invitations {ARROW}</a><a class="text-link" href="https://lists.rationality-munich.com/archive">See the archive →</a></div><div class="newsletter-note"><p>EA, RATIONALITY,<br>OR EVERYTHING.</p><span>Pick what you want invitations about. Unsubscribe at any time.</span></div></section>
</main>"##,
        count = count,
        plans = plans::section(),
        groups = groups::section(),
        more = groups::more(),
    )
}

fn calendar(events: &[&Event], today: NaiveDate) -> String {
    let grids: String = current_and_next(today)
        .iter()
        .map(|&(year, month)| {
            let n = month_count(events, year, month);
            format!(
                r#"<section class="calendar-month"><div class="month-heading"><h2>{label}</h2><span>{n} event{s}</span></div>{grid}</section>"#,
                label = NaiveDate::from_ymd_opt(year, month, 1)
                    .expect("a first of the month is real")
                    .format("%B <span>%Y</span>"),
                s = plural(n),
                grid = month_grid(year, month, events, today),
            )
        })
        .collect();
    let rows: String = events.iter().map(|e| event_row(e)).collect();
    let list = if events.is_empty() {
        r#"<div class="empty-state"><h2>Nothing listed.</h2><p>No upcoming events are announced right now. The groups post as plans firm up, or ask them directly.</p><div class="actions"><a class="button primary" href="{AT}/subscribe/">Get event invitations</a><a class="text-link" href="{AT}/#community">Explore the groups ↗</a></div></div>"#.to_string()
    } else {
        format!(
            r#"<section class="calendar-month"><div class="month-heading"><h2>Every upcoming event</h2><span>{n} event{s}</span></div>{rows}</section>"#,
            n = events.len(),
            s = plural(events.len()),
        )
    };
    let feed = format!(
        "rationality-munich.com/calendar/feeds/{}.ics",
        DEFAULT_GROUPS.join("+")
    );
    format!(
        r##"<main id="main" class="wrap calendar-page">
<div class="page-heading"><p class="eyebrow">CALENDAR</p><div class="title-actions"><h1>Upcoming events</h1></div><p>Discussions, dinners, games and talks in Munich.</p>
<details class="subscribe-box"><summary>Subscribe to the calendar</summary><div><p>Every event below, in your own calendar app, updated automatically.</p><a class="button primary" href="webcal://{feed}">Open in Apple Calendar or Outlook {ARROW}</a><h3>Google Calendar</h3><p>Other calendars → + → From URL, then paste this address.</p><h3>Outlook on the web</h3><p>Add calendar → Subscribe from web, then paste this address.</p><code>https://{feed}</code><p><a class="text-link" href="/calendar/rss.xml">RSS</a> and <a class="text-link" href="/calendar/feed.xml">Atom</a> feeds too.</p></div></details></div>
<div class="calendar-context"><span><span class="status-dot"></span> {n} upcoming event{s}</span><span>All times Munich local time</span></div>
{grids}
{list}
<aside class="coverage-note"><h3>A note on coverage</h3><p>These are the events the groups have announced on LessWrong, the EA Forum, Meetup, Luma and their own pages. Check the original listing for registration, language and cost. Events later than next month are in the list above but not in the grids.</p><div class="actions"><a class="text-link" href="/calendar/past/">Past events</a><a class="text-link" href="/calendar/stats/">Statistics</a><a class="text-link" href="mailto:rationality@hilll.dev?subject=Event%20suggestion%20for%20Rationality%20Munich">Suggest an event</a></div></aside>
</main>"##,
        n = events.len(),
        s = plural(events.len()),
    )
}

fn event_page(e: &Event, now: DateTime<Utc>) -> String {
    let start = e.start.with_timezone(&Berlin);
    let more: String = e
        .links
        .iter()
        .skip(1)
        .map(|(label, url)| {
            format!(
                r#"<a class="text-link" href="{url}">Also announced on {label} ↗</a>"#,
                url = html_escape(url),
                label = html_escape(label),
            )
        })
        .collect();
    let announcement = match source(e) {
        Some((label, url)) => format!(
            r#"<a class="button primary" href="{url}">Details &amp; RSVP on {label} {ARROW}</a>"#,
            url = html_escape(url),
            label = html_escape(label),
        ),
        None => r#"<p class="small-note">This one is organised in a group chat; there is no public announcement to link to.</p>"#.to_string(),
    };
    let signups = if e.signed_up() > 0 {
        format!(
            r#"<div class="detail-fact"><span>{} signed up so far</span></div>"#,
            e.signed_up()
        )
    } else {
        String::new()
    };
    let lede = if e.excerpt.is_empty() {
        String::new()
    } else {
        format!(r#"<p class="detail-lead">{}</p>"#, html_escape(&e.excerpt))
    };
    format!(
        r##"<main id="main" class="wrap detail-page">
<a class="text-link back-link" href="{AT}/calendar/">← All events</a>
<div class="detail-grid"><article><div class="detail-labels"><span class="tag">{category}</span><span class="eyebrow">{groups}</span></div><h1>{title}</h1>{lede}
<div class="detail-body"><h2>What to know before you go</h2><p>This event is organised by {groups}. The full announcement has the latest information and any registration instructions.</p>
<p>Events are in English, unless the announcement says otherwise; PauseAI Munich’s are sometimes in German. Check the announcement for cost, registration, capacity, preparation and step-free access, or ask the host.</p>
<a class="text-link" href="mailto:rationality@hilll.dev?subject=Correction">Report a detail that needs updating ↗</a></div></article>
<aside class="detail-aside"><p class="eyebrow">PRACTICAL DETAILS</p>
<div class="detail-fact"><time datetime="{iso}">{long}</time></div>
<div class="detail-fact"><span>{time}<small>Munich local time · Europe/Berlin</small></span></div>
<div class="detail-fact"><span>{place}</span></div>
<div class="detail-fact"><span>{groups}</span></div>
{signups}
{announcement}
<a class="button secondary" href="/calendar/e/{id}.ics" download="{id}.ics">Add this event to my calendar</a>
<p class="small-note">The .ics adds this event only. Subscribe to the calendar to get later changes.</p>
{more}
<p class="snapshot-note">Read from the organisers’ announcements at {updated}. Check the original for changes.</p></aside></div>
</main>"##,
        category = html_escape(groups::category(e)),
        groups = html_escape(&group_list(e)),
        title = html_escape(&e.title),
        iso = start.format("%Y-%m-%d"),
        long = start.format("%A, %-d %B %Y"),
        time = html_escape(&clock(e)),
        place = html_escape(place(e)),
        id = e.id,
        updated = now.with_timezone(&Berlin).format("%-d %b %Y, %H:%M"),
    )
}

fn subscribe() -> String {
    format!(
        r##"<main id="main" class="wrap subscribe-page">
<div class="subscribe-copy"><p class="eyebrow">MAILING LIST</p><h1>Event invites by email</h1>
<p>An email when the Munich groups announce something in the topics you picked.</p>
<ul><li>Choose the topics you care about</li><li>Change your preferences whenever you like</li><li>Unsubscribe with a link in every email</li></ul>
<div class="signup-expectations"><h3>What to expect</h3><p>Invitations to community events. No open or click tracking, and you can delete your data from any email.</p><a class="text-link" href="https://lists.rationality-munich.com/archive">Read the archive {ARROW}</a></div></div>
<div class="signup-card"><h2>Sign up</h2><p class="form-intro">The list runs on this site’s own server. The signup form is part of it, not of this page.</p>
<a class="button primary submit-button" href="https://lists.rationality-munich.com/subscription/form">Go to the signup form {ARROW}</a>
<p class="small-note">This page does not take your address; the link goes to the mailing list this site already runs. <a href="{AT}/privacy/">Privacy details</a></p></div>
</main>"##
    )
}

fn privacy() -> String {
    // www/privacy.html, which goes when the hub does. Copied unchanged,
    // including the date: changing a privacy policy to fit a layout is wrong.
    r##"<main id="main" class="wrap reading-page">
<div class="page-heading"><p class="eyebrow">PRIVACY</p><h1>What this site stores</h1><p>Datenschutzerklärung · last updated 23 September 2026</p></div>
<article class="prose"><p>This site has no cookies, no analytics and no tracking. The mailing list stores your email address and the lists you picked, and you can delete that yourself at any time.</p>
<h2>Who is responsible</h2><p>L. David Weil, for Rationality Munich, an informal, non-commercial community group. Contact: <a href="mailto:rationality@hilll.dev">rationality@hilll.dev</a>.</p>
<h2>Visiting this website</h2><p>When you load a page, the web server logs your IP address, the time, the page requested, the referring page and your browser’s user agent. We use these logs only to keep the server running and to deal with abuse, which is our legitimate interest (Art. 6(1)(f) GDPR). They are deleted after 7 days.</p>
<p>The site sets no cookies and loads nothing from other servers: no fonts, no scripts, no analytics. The pages are plain files, written once an hour by a job on this server that reads the groups’ announcements; your browser only ever talks to rationality-munich.com.</p>
<p>The links to other groups, such as LessWrong, the EA Forum, Luma, Meetup and WhatsApp, take you to services run by others, with their own privacy policies. Nothing is sent to them until you click.</p>
<h2>The mailing list</h2><p>If you subscribe at <a href="https://lists.rationality-munich.com/subscription/form">lists.rationality-munich.com</a>, we store:</p>
<ul><li>your email address, and your name if you give one</li><li>which lists you subscribed to (EA, rationality, other events)</li><li>when you subscribed and when you confirmed your subscription</li></ul>
<p>We don’t store your IP address, and we don’t track whether you open our emails or click links in them.</p>
<p>We use this only to send you invitations to events on the lists you picked. The legal basis is your consent (Art. 6(1)(a) GDPR). You can withdraw it at any time with the link at the bottom of every email. That link also lets you change your lists, download your data, or delete it completely.</p>
<p>We keep your data until you unsubscribe or delete it. We remove addresses of people who have unsubscribed from time to time, and straight away if you ask.</p>
<h2>Attendance reports</h2><p>On the <a href="/calendar/past/">past events page</a> you can say roughly how many people came to an event. We store the event, the number and the time, and nothing about you. Your IP address is used only briefly, in memory, to stop one address from sending too many reports, and is forgotten within a day.</p>
<h2>Who else handles your data</h2><p>We don’t sell or share your data. Two companies process it for us, each under a data processing agreement (Art. 28 GDPR), and both keep it in the EU:</p>
<ul><li>Hetzner Online GmbH, Gunzenhausen, Germany: hosts the server this website and the mailing list run on.</li><li>Lettermint, Netherlands: will deliver our emails once we start sending. For this it gets your email address and the content of each email.</li></ul>
<h2>Your rights</h2><p>You can ask us for a copy of your data, to correct it, to delete it, to restrict how we use it, or to get it in a portable format. You can also object to how we process it (Art. 15–21 GDPR). Email <a href="mailto:rationality@hilll.dev">rationality@hilll.dev</a> and we’ll answer within a month.</p>
<p>You also have the right to complain to a data protection authority. Ours is the Bayerisches Landesamt für Datenschutzaufsicht (BayLDA) in Ansbach.</p></article>
</main>"##
        .to_string()
}

/// The 404 page. nginx serves it for anything missing under `AT`.
fn not_found() -> String {
    format!(
        r##"<main id="main" class="wrap empty-state">
<p class="eyebrow">404</p><h1>Page not found</h1>
<p>It may have been an event that has already happened, or a mistyped address. The calendar lists everything still to come.</p>
<div class="actions"><a class="button primary" href="{AT}/calendar/">See the calendar {ARROW}</a><a class="text-link" href="{AT}/">Home</a></div>
</main>"##
    )
}

/// Ignored while this sits under /demo, since crawlers only read the one at
/// the site root, and correct once it moves.
fn robots() -> String {
    format!(
        "User-agent: *\n\
         Allow: /\n\
         # One calendar file per event and per group combination: nothing to index.\n\
         Disallow: /calendar/e/\n\
         Disallow: /calendar/feeds/\n\
         \n\
         Sitemap: {SITE}{AT}/sitemap.xml\n"
    )
}

/// Every page worth indexing, rewritten each run, so the event pages are
/// listed and stay current.
fn sitemap(events: &[&Event], now: DateTime<Utc>) -> String {
    let day = now.with_timezone(&Berlin).format("%Y-%m-%d");
    let mut urls = vec![
        (String::new(), "hourly"),
        ("calendar/".to_string(), "hourly"),
        ("tools/".to_string(), "monthly"),
        ("subscribe/".to_string(), "monthly"),
        ("about/".to_string(), "monthly"),
        ("privacy/".to_string(), "yearly"),
        ("impressum/".to_string(), "yearly"),
    ];
    urls.extend(
        events
            .iter()
            .map(|e| (format!("events/{}/", e.id), "daily")),
    );
    let body: String = urls
        .iter()
        .map(|(path, freq)| {
            format!(
                "  <url><loc>{SITE}{AT}/{path}</loc><lastmod>{day}</lastmod><changefreq>{freq}</changefreq></url>\n"
            )
        })
        .collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n\
         {body}</urlset>\n"
    )
}

/// Rebuilt hourly, so the expiry stays a year away instead of lapsing.
fn security(now: DateTime<Utc>) -> String {
    let expires = now + Duration::days(365);
    format!(
        "Contact: mailto:rationality@hilll.dev\n\
         Expires: {}\n\
         Preferred-Languages: en, de\n\
         Canonical: {SITE}{AT}/.well-known/security.txt\n",
        expires.format("%Y-%m-%dT%H:%M:%S.000Z"),
    )
}

fn impressum() -> String {
    // www/impressum.html. Required by § 5 DDG, so this page carries its own
    // copy rather than linking to one that is going away.
    r##"<main id="main" class="wrap reading-page">
<div class="page-heading"><p class="eyebrow">LEGAL NOTICE</p><h1>Impressum</h1></div>
<article class="prose"><h2>Angaben gemäß § 5 DDG</h2><p>L. David Weil</p>
<h2>Kontakt</h2><p>E-Mail: <a href="mailto:rationality@hilll.dev">rationality@hilll.dev</a></p>
<h2>Verantwortlich für den Inhalt nach § 18 Abs. 2 MStV</h2><p>L. David Weil</p>
<p>Rationality Munich is an informal, non-commercial community of people interested in rationality, effective altruism and AI safety. It is not a registered association. The external groups linked from this site are run by their own organisers, who are responsible for their content.</p></article>
</main>"##
        .to_string()
}

fn about(now: DateTime<Utc>) -> String {
    format!(
        r##"<main id="main" class="wrap reading-page">
<div class="page-heading"><p class="eyebrow">ABOUT</p><h1>About this demo</h1><p>A hub for the rationality, effective altruism, AI safety and philosophy groups in Munich, and a calendar of everything they have announced.</p></div>
<article class="prose"><h2>Where the events come from</h2><p>A job on this server reads the groups’ announcements on LessWrong, the EA Forum, Meetup, Luma, Philosophia’s calendar and a couple of pages, once an hour, merges the ones that were posted in several places, and writes these pages. Nothing here is a copy kept by hand. This page was written at {updated} and will be written again within the hour.</p>
<p>It is plain HTML and one stylesheet, with no script. The pages work with JavaScript off and in a text browser, and loading one never waits on another site.</p>
<h2>What is here</h2><ul><li>This page: what is on, the six groups, and the further Munich groups whose events are in the calendar but off by default.</li><li>A calendar: this month and next as week grids, then every announced event as a list.</li><li>A page per event, with the organiser’s announcement and an .ics download.</li></ul>
<h2>What is not here yet</h2>
<ul><li>Filtering the calendar by group. <a href="/calendar/past/">Past events</a> and the <a href="/calendar/stats/">statistics</a> are still in the previous design.</li>
<li>Organisers’ approval of how their groups are described. Every line of that was written for the previous version of this site, but nobody from the groups has been asked to check it.</li></ul>
<h2>Photo &amp; artwork credits</h2><p>The photograph shows Munich’s Englischer Garten; it is location imagery, not a photograph of the community.</p>
<p><a href="https://commons.wikimedia.org/wiki/File:Monopteros_in_Englischer_Garten,_Munich.JPG">Monopteros in Englischer Garten, Munich</a> by High Contrast (2013), licensed under <a href="https://creativecommons.org/licenses/by/3.0/de/deed.en">CC BY 3.0 Germany</a>. Optimised and cropped for this layout.</p>
<p>The social-sharing card is AI-generated typographic artwork made for this demo.</p></article>
</main>"##,
        updated = now.with_timezone(&Berlin).format("%-d %b %Y, %H:%M"),
    )
}

// ----------------------------------------------------------------- writing

fn write(path: &Path, text: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, text)
}

/// Writes the whole demo into `dir`. `events` is every event the run knows
/// about, in start order.
pub fn build(
    dir: &Path,
    events: &[Event],
    stale: &[String],
    now: DateTime<Utc>,
) -> std::io::Result<()> {
    let today = now.with_timezone(&Berlin).date_naive();
    // The same events the hub shows by default, still to come.
    let upcoming: Vec<&Event> = events
        .iter()
        .filter(|e| e.end_or_default() >= now && e.in_any(DEFAULT_GROUPS))
        .collect();

    write(&dir.join("style.css"), CSS)?;
    write(&dir.join("robots.txt"), &robots())?;
    write(&dir.join("sitemap.xml"), &sitemap(&upcoming, now))?;
    write(&dir.join(".well-known/security.txt"), &security(now))?;
    write(
        &dir.join("404.html"),
        &shell(
            Shell {
                title: "Not found · Rationality Munich".into(),
                description: "That page is not here.".into(),
                canonical: format!("{SITE}{AT}/404.html"),
                body: not_found(),
            },
            now,
            stale,
        ),
    )?;
    write(&dir.join("favicon.svg"), FAVICON)?;
    fs::write(dir.join("munich.webp"), PHOTO)?;
    fs::write(dir.join("og.png"), OG)?;

    for (path, shell_args) in [
        (
            "index.html",
            Shell {
                title: "Rationality Munich — events, groups and meetups".into(),
                description: "Munich's rationality, effective altruism, philosophy and AI safety groups, and everything they have coming up.".into(),
                canonical: format!("{SITE}{AT}/"),
                body: home(&upcoming, today),
            },
        ),
        (
            "calendar/index.html",
            Shell {
                title: "Upcoming events · Rationality Munich".into(),
                description: "Every event the Munich rationality, EA and AI safety groups have announced, this month and next as a calendar.".into(),
                canonical: format!("{SITE}{AT}/calendar/"),
                body: calendar(&upcoming, today),
            },
        ),
        (
            "tools/index.html",
            Shell {
                title: "Tools · Rationality Munich".into(),
                description: "AnkiQuest, and the Petrov and Arkhipov Day ceremonies: things this site hosts, free for other groups to use.".into(),
                canonical: format!("{SITE}{AT}/tools/"),
                body: tools::page(),
            },
        ),
        (
            "subscribe/index.html",
            Shell {
                title: "Event invitations · Rationality Munich".into(),
                description: "Get an email when the Munich groups announce an event in the topics you pick.".into(),
                canonical: format!("{SITE}{AT}/subscribe/"),
                body: subscribe(),
            },
        ),
        (
            "privacy/index.html",
            Shell {
                title: "Privacy · Rationality Munich".into(),
                description: "No cookies, no analytics, no tracking. What the mailing list stores, and how to delete it.".into(),
                canonical: format!("{SITE}{AT}/privacy/"),
                body: privacy(),
            },
        ),
        (
            "impressum/index.html",
            Shell {
                title: "Impressum · Rationality Munich".into(),
                description: "Legal notice for rationality-munich.com.".into(),
                canonical: format!("{SITE}{AT}/impressum/"),
                body: impressum(),
            },
        ),
        (
            "about/index.html",
            Shell {
                title: "About this demo · Rationality Munich".into(),
                description: "A design proposal for rationality-munich.com, built from the live calendar every hour.".into(),
                canonical: format!("{SITE}{AT}/about/"),
                body: about(now),
            },
        ),
    ] {
        write(&dir.join(path), &shell(shell_args, now, stale))?;
    }

    for e in &upcoming {
        write(
            &dir.join(format!("events/{}/index.html", e.id)),
            &shell(
                Shell {
                    title: format!("{} · Rationality Munich", e.title),
                    description: if e.excerpt.is_empty() {
                        "An event from one of Munich's rationality, EA and AI safety groups"
                            .to_string()
                    } else {
                        e.excerpt.clone()
                    },
                    canonical: format!("{SITE}{AT}/events/{}/", e.id),
                    body: event_page(e, now),
                },
                now,
                stale,
            ),
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{assign_ids, sanitize, tests::ev};

    fn sample() -> Vec<Event> {
        let mut events = sanitize(vec![ev("Community Dinner", 0, "LessWrong")]);
        assign_ids(&mut events);
        events
    }

    #[test]
    fn a_month_is_whole_weeks_starting_on_monday() {
        // October 2026 starts on a Thursday and has 31 days: five rows.
        let weeks = weeks_of(2026, 10);
        assert_eq!(weeks.len(), 5);
        for week in &weeks {
            assert_eq!(week.len(), 7);
            assert_eq!(week[0].weekday(), chrono::Weekday::Mon);
        }
        assert_eq!(weeks[0][0], NaiveDate::from_ymd_opt(2026, 9, 28).unwrap());
        assert_eq!(weeks[4][6], NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
        // Every day of the month appears exactly once.
        let inside = weeks.concat().iter().filter(|d| d.month() == 10).count();
        assert_eq!(inside, 31);
    }

    #[test]
    fn february_in_a_leap_year_fits_and_december_rolls_over() {
        assert_eq!(
            weeks_of(2032, 2)
                .concat()
                .iter()
                .filter(|d| d.month() == 2)
                .count(),
            29
        );
        assert_eq!(
            current_and_next(NaiveDate::from_ymd_opt(2026, 12, 9).unwrap()),
            [(2026, 12), (2027, 1)]
        );
    }

    #[test]
    fn an_event_lands_in_its_own_day_and_links_to_its_page() {
        let events = sample();
        let upcoming: Vec<&Event> = events.iter().collect();
        let today = NaiveDate::from_ymd_opt(2026, 9, 10).unwrap();
        let grid = month_grid(2026, 9, &upcoming, today);
        assert!(grid.contains(&format!(
            r#"href="{AT}/events/2026-09-26-community-dinner/""#
        )));
        assert!(grid.contains(r#"<td class="day busy"><span class="day-number">26"#));
        // Today is marked, and only today.
        assert_eq!(grid.matches(r#"class="day today""#).count(), 1);
        assert!(grid.contains(r#"<span class="day-number">10<span class="sr-only"> — today"#));
    }

    #[test]
    fn nothing_from_a_feed_reaches_the_page_unescaped() {
        let mut nasty = ev("<script>alert(1)</script>", 0, "A");
        nasty.excerpt = "<img src=x onerror=alert(1)>".into();
        nasty.location = "\"><script>".into();
        let mut events = sanitize(vec![nasty]);
        assign_ids(&mut events);
        let upcoming: Vec<&Event> = events.iter().collect();
        let now = events[0].start;
        let today = now.with_timezone(&Berlin).date_naive();
        for html in [
            home(&upcoming, today),
            calendar(&upcoming, today),
            event_page(upcoming[0], now),
        ] {
            assert!(!html.contains("<script>alert"), "{html}");
            assert!(!html.contains("<img src=x"));
        }
    }

    /// Every link the demo makes to itself goes through `AT`, so that setting
    /// `AT` to "" moves all of them at once.
    #[test]
    fn nothing_spells_the_prefix_out_by_hand() {
        let events = sample();
        let upcoming: Vec<&Event> = events.iter().collect();
        let now = events[0].start;
        let today = now.with_timezone(&Berlin).date_naive();
        let pages = [
            PAGE.replace("{{at}}", AT),
            home(&upcoming, today),
            calendar(&upcoming, today),
            event_page(upcoming[0], now),
            subscribe(),
            privacy(),
            impressum(),
            about(now),
            groups::section(),
            groups::more(),
        ];
        for html in &pages {
            for link in html
                .split("href=\"")
                .skip(1)
                .filter_map(|s| s.split('"').next())
            {
                if !link.starts_with('/') {
                    continue;
                }
                // /calendar is this program's other output and keeps its
                // own URLs.
                assert!(
                    link.starts_with(AT) || link.starts_with("/calendar"),
                    "{link} is not under {AT}"
                );
                // The old prefix, in case one got left behind.
                assert!(!link.starts_with("/demo"), "{link} still says /demo");
            }
        }
    }

    /// feed.xml, rss.xml and every .ics point at /calendar#<id>, and have for
    /// as long as people have been subscribing. The rows keep those anchors.
    #[test]
    fn the_calendar_keeps_the_anchors_the_feeds_link_to() {
        let events = sample();
        let upcoming: Vec<&Event> = events.iter().collect();
        let today = upcoming[0].start.with_timezone(&Berlin).date_naive();
        let html = calendar(&upcoming, today);
        for e in &upcoming {
            assert!(
                html.contains(&format!(r#"<article id="{}""#, e.id)),
                "{} has no anchor",
                e.id
            );
        }
    }

    #[test]
    fn an_empty_calendar_still_renders_both_pages() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        let home = home(&[], today);
        assert!(home.contains("Nothing is announced"));
        assert!(calendar(&[], today).contains("Nothing listed."));
    }
}
