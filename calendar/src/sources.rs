//! Where events come from. Each source returns everything it knows, past and
//! upcoming; the caller decides what goes where.

use std::collections::HashSet;
use std::error::Error;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::event::{Came, Event, Raw, title_words};
use crate::ics;

pub type Res<T> = Result<T, Box<dyn Error>>;

const UA: &str =
    "Mozilla/5.0 (compatible; rationality-munich-calendar/1.0; +https://rationality-munich.com)";

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(UA)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into()
}

fn get(agent: &ureq::Agent, url: &str) -> Res<String> {
    Ok(agent.get(url).call()?.body_mut().read_to_string()?)
}

type Fetch = Box<dyn Fn(&ureq::Agent, DateTime<Utc>) -> Res<Vec<Event>>>;

pub struct Source {
    /// Cache file name.
    pub key: &'static str,
    /// Shown when the source cannot be reached.
    pub label: &'static str,
    pub fetch: Fetch,
}

pub fn all() -> Vec<Source> {
    let forum = |key, label, endpoint, group_id, group| Source {
        key,
        label,
        fetch: Box::new(move |agent, _| forum_events(agent, endpoint, group_id, group, label)),
    };
    vec![
        forum("lw-acx", "LessWrong", LESSWRONG, "EBvaNj4oAn5nkkzbJ", "acx"),
        // The group's predecessor, inactive since 2021; history only.
        forum(
            "lw-acx-old",
            "LessWrong",
            LESSWRONG,
            "QGxgyBgHCcKiE8p6p",
            "acx",
        ),
        forum("lw-ea", "LessWrong", LESSWRONG, "cavvnnKLnWeqHPAsR", "ea"),
        forum("eaforum", "EA Forum", EA_FORUM, "E8ruG2KzaNpynpGXK", "ea"),
        meetup_group("meetup", "effective-altruism-munich", "ea", |_| true),
        Source {
            key: "luma",
            label: "Luma",
            fetch: Box::new(|agent, _| luma_events(agent)),
        },
        Source {
            key: "philosophia",
            label: "Philosophia",
            fetch: Box::new(philosophia_events),
        },
        Source {
            key: "mlphil",
            label: "MCMP",
            fetch: Box::new(|agent, _| mlphil_events(agent)),
        },
        // The GEB reading group has no page of its own; it posts in a general
        // Munich activities group, so only its events are taken.
        // Its events are titled "Bi-Weekly GEB \"Cafminar\"" and the like.
        meetup_group("geb", "munich-weekly-activities", "geb", |title| {
            let t = title.to_lowercase();
            t.split(|c: char| !c.is_alphanumeric()).any(|w| w == "geb")
                || t.contains("gödel")
                || t.contains("escher")
        }),
        meetup_group(
            "agi",
            "munchen-artificial-general-intelligence-meetup-group",
            "agi",
            |_| true,
        ),
        meetup_group(
            "skeptics",
            "skeptics-in-the-pub-munchen",
            "skeptics",
            |_| true,
        ),
        meetup_group("science", "science-club-munich", "science", |_| true),
        meetup_group("minds", "minds-in-motion-munich", "minds", |_| true),
        meetup_group("curious", "lifelong__curious", "curious", |_| true),
        meetup_group("silentbooks", "silent-book-club", "silentbooks", |_| true),
        meetup_group("culture", "culture-club-munich", "culture", |_| true),
    ]
}

/// One Meetup group; `keep` picks the events that are the group's when it
/// posts through a broader one.
fn meetup_group(
    key: &'static str,
    slug: &'static str,
    group: &'static str,
    keep: fn(&str) -> bool,
) -> Source {
    Source {
        key,
        label: "Meetup",
        fetch: Box::new(move |agent, _| meetup_events(agent, slug, group, keep)),
    }
}

// LessWrong and the EA Forum run the same software and the same GraphQL API.

const LESSWRONG: &str = "https://www.lesswrong.com/graphql";
const EA_FORUM: &str = "https://forum.effectivealtruism.org/graphql";

const FORUM_QUERY: &str = r#"
{ posts(input: {terms: {view: "VIEW", groupId: "GROUP", limit: 500}}) {
    results { title startTime endTime location onlineEvent pageUrl
              contents { plaintextDescription } rsvps } } }
"#;

/// The names on a post's "yes" RSVPs.
fn forum_yes(rsvps: &Value) -> Vec<&str> {
    rsvps.as_array().map_or(Vec::new(), |list| {
        list.iter()
            .filter(|r| r["response"] == "yes")
            .filter_map(|r| r["name"].as_str())
            .collect()
    })
}

fn forum_time(v: &Value) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(v.as_str()?)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

fn forum_events(
    agent: &ureq::Agent,
    endpoint: &str,
    group_id: &str,
    group: &str,
    label: &str,
) -> Res<Vec<Event>> {
    let mut out = Vec::new();
    for view in ["upcomingEvents", "pastEvents"] {
        let query = FORUM_QUERY.replace("VIEW", view).replace("GROUP", group_id);
        let data: Value = serde_json::from_str(
            &agent
                .post(endpoint)
                .header("Content-Type", "application/json")
                .send(json!({ "query": query }).to_string())?
                .body_mut()
                .read_to_string()?,
        )?;
        if let Some(err) = data["errors"].get(0) {
            return Err(format!("{endpoint}: {}", err["message"]).into());
        }
        for p in data["data"]["posts"]["results"]
            .as_array()
            .ok_or("no results")?
        {
            let Some(start) = forum_time(&p["startTime"]) else {
                continue;
            };
            let mut e = Event::new(Raw {
                title: p["title"].as_str().unwrap_or(""),
                start,
                end: forum_time(&p["endTime"]),
                location: p["location"].as_str().unwrap_or(""),
                online: p["onlineEvent"].as_bool().unwrap_or(false),
                text: p["contents"]["plaintextDescription"].as_str().unwrap_or(""),
                group,
                label,
                url: p["pageUrl"].as_str().unwrap_or(""),
            });
            let yes = forum_yes(&p["rsvps"]);
            e.signups = yes.len() as u32;
            e.sign_up(yes);
            out.push(e);
        }
    }
    Ok(out)
}

// Meetup: the GraphQL endpoint its own website uses. No key needed; it has
// every group's full history, newest first, with venues.

const MEETUP_GQL: &str = "https://www.meetup.com/gql2";

const MEETUP_QUERY: &str = r#"
query($group: String!, $status: EventStatus!, $after: String) {
  groupByUrlname(urlname: $group) {
    events(status: $status, first: 100, after: $after, sort: DESC) {
      pageInfo { hasNextPage endCursor }
      edges { node { title dateTime endTime eventUrl isOnline description
                     venue { name address } going { totalCount }
                     rsvps(first: 100) { edges { node { status member { name } } } } } }
    }
  }
}
"#;

/// At most this many pages of a hundred per group and run. Culture Club alone
/// has held almost 700 events since 2015.
const MEETUP_PAGES: usize = 10;

/// A Meetup group's upcoming (`ACTIVE`) and past events. Cancelled ones have
/// their own status and never come back.
fn meetup_events(
    agent: &ureq::Agent,
    slug: &str,
    group: &str,
    keep: fn(&str) -> bool,
) -> Res<Vec<Event>> {
    let mut out = Vec::new();
    for status in ["ACTIVE", "PAST"] {
        let mut after = Value::Null;
        for _ in 0..MEETUP_PAGES {
            let body = json!({
                "query": MEETUP_QUERY,
                "variables": { "group": slug, "status": status, "after": after },
            });
            let data: Value = serde_json::from_str(
                &agent
                    .post(MEETUP_GQL)
                    .header("Content-Type", "application/json")
                    .send(body.to_string())?
                    .body_mut()
                    .read_to_string()?,
            )?;
            if let Some(err) = data["errors"].get(0) {
                return Err(format!("meetup {slug}: {}", err["message"]).into());
            }
            let events = &data["data"]["groupByUrlname"]["events"];
            if events.is_null() {
                return Err(format!("meetup {slug}: no such group").into());
            }
            for edge in events["edges"].as_array().ok_or("no edges")? {
                if let Some(e) = meetup_event(&edge["node"], group, keep) {
                    out.push(e);
                }
            }
            if events["pageInfo"]["hasNextPage"] != true {
                break;
            }
            after = events["pageInfo"]["endCursor"].clone();
        }
    }
    Ok(out)
}

fn meetup_event(e: &Value, group: &str, keep: fn(&str) -> bool) -> Option<Event> {
    let title = e["title"].as_str()?;
    let url = e["eventUrl"].as_str()?;
    let start = forum_time(&e["dateTime"])?;
    if !keep(title) || !url.starts_with("https://www.meetup.com/") {
        return None;
    }
    let mut place: Vec<&str> = Vec::new();
    for x in [&e["venue"]["name"], &e["venue"]["address"]]
        .iter()
        .filter_map(|x| x.as_str())
    {
        // Venues are often named after their own address; online events
        // come with a placeholder venue.
        if x.is_empty() || x == "Online event" || place.iter().any(|p| p.contains(x)) {
            continue;
        }
        place.retain(|p| !x.contains(p));
        place.push(x);
    }
    // Descriptions are Markdown; the excerpt wants plain text.
    let text = e["description"]
        .as_str()
        .unwrap_or("")
        .replace(['*', '#', '_'], "");
    let mut event = Event::new(Raw {
        title,
        start,
        end: forum_time(&e["endTime"]),
        location: &place.join(", "),
        online: e["isOnline"].as_bool().unwrap_or(false),
        text: &text,
        group,
        label: "Meetup",
        url,
    });
    event.signups = e["going"]["totalCount"].as_u64().unwrap_or(0) as u32;
    // Groups that hide their members return no names; the count stands.
    if let Some(edges) = e["rsvps"]["edges"].as_array() {
        event.sign_up(
            edges
                .iter()
                .filter(|x| x["node"]["status"] == "YES")
                .filter_map(|x| x["node"]["member"]["name"].as_str()),
        );
    }
    Some(event)
}

// The MCMP's philosophy of machine learning reading group keeps its schedule,
// back to 2022, as one table per semester.

const MLPHIL_PAGE: &str = "https://tomster.userweb.mwn.de/mlregr/";

/// Inner text of every `<td>`, in order; tags dropped.
fn table_cells(html: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("<td") {
        rest = &rest[i..];
        let Some(open) = rest.find('>') else { break };
        let Some(close) = rest.find("</td>") else {
            break;
        };
        let inner = &rest[open + 1..close.max(open + 1)];
        cells.push(collapse(&strip_html(inner)));
        rest = &rest[close.max(open + 1)..];
    }
    cells
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// "Fri 17 Jul, 16-17" under a semester heading, as Munich times.
fn mlphil_time(cell: &str, heading: &str) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    use chrono::TimeZone;
    use chrono_tz::Europe::Berlin;
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (date, hours) = cell.split_once(',')?;
    let mut words = date.split_whitespace().skip(1);
    let day: u32 = words.next()?.parse().ok()?;
    let word = words.next()?;
    let month = MONTHS.iter().position(|m| word.starts_with(m))? as u32 + 1;
    // "Summer 2026", or "Winter 2025/2026" / "Winter 2023/24": winter months
    // from September on belong to the first year.
    let first: i32 = heading
        .split_whitespace()
        .nth(1)?
        .split('/')
        .next()?
        .parse()
        .ok()?;
    let year = if heading.starts_with("Winter") && month < 9 {
        first + 1
    } else {
        first
    };
    let clock = |t: &str| -> Option<(u32, u32)> {
        let mut p = t.trim().split(['.', ':']);
        let h = p.next()?.parse().ok()?;
        let m = match p.next() {
            Some(m) => m.parse().ok()?,
            None => 0,
        };
        Some((h, m))
    };
    let (from, to) = hours.split_once(['-', '–'])?;
    let at = |(h, m): (u32, u32)| {
        Berlin
            .with_ymd_and_hms(year, month, day, h, m, 0)
            .earliest()
            .map(|t| t.with_timezone(&Utc))
    };
    Some((at(clock(from)?)?, at(clock(to)?)?))
}

fn mlphil_events(agent: &ureq::Agent) -> Res<Vec<Event>> {
    let page = get(agent, MLPHIL_PAGE)?;
    let mut out = Vec::new();
    // Each semester: an <h4> heading, then its table.
    for section in page.split("<h4").skip(1) {
        let Some((heading, _)) = section
            .split_once('>')
            .and_then(|(_, r)| r.split_once("</h4>"))
        else {
            continue;
        };
        let heading = collapse(&strip_html(heading));
        if !heading.starts_with("Summer") && !heading.starts_with("Winter") {
            continue;
        }
        for row in table_cells(section).chunks(2) {
            let [date, reading] = row else { continue };
            let Some((start, end)) = mlphil_time(date, &heading) else {
                continue;
            };
            out.push(Event::new(Raw {
                title: reading,
                start,
                end: Some(end),
                location: "Ludwigstraße 31, room 028",
                online: false,
                text: "",
                group: "mlphil",
                label: "Reading group",
                url: MLPHIL_PAGE,
            }));
        }
    }
    Ok(out)
}

// Luma: an iCalendar feed of past and upcoming events.

const LUMA_ICS: &str = "https://api.lu.ma/ics/get?entity=calendar&id=cal-2OD0oFnuk1CJPbr";

fn find_luma_link(text: &str) -> Option<String> {
    ["https://luma.com/", "https://lu.ma/"]
        .iter()
        .find_map(|prefix| {
            let i = text.find(prefix)?;
            let slug: String = text[i + prefix.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
                .collect();
            (!slug.is_empty()).then(|| format!("{prefix}{slug}"))
        })
}

fn luma_events(agent: &ureq::Agent) -> Res<Vec<Event>> {
    let mut out = Vec::new();
    for e in ics::parse(&get(agent, LUMA_ICS)?) {
        let Some(start) = ics::time(ics::prop(&e, "DTSTART")) else {
            continue;
        };
        let text = ics::value(&e, "DESCRIPTION");
        let url = Some(ics::value(&e, "URL"))
            .filter(|u| !u.is_empty())
            .or_else(|| find_luma_link(&text))
            .unwrap_or_else(|| "https://luma.com/eamunich".into());
        let location = ics::value(&e, "LOCATION");
        let online = location.starts_with("http");
        // Luma's description is only boilerplate (link, address, host), so no excerpt.
        let mut event = Event::new(Raw {
            title: &ics::value(&e, "SUMMARY"),
            start,
            end: ics::time(ics::prop(&e, "DTEND")),
            location: if online { "" } else { &location },
            online,
            text: "",
            group: "ea",
            label: "Luma",
            url: &url,
        });
        if let Some((count, names)) = luma_guests(agent, &url) {
            event.signups = count;
            event.sign_up(names.iter().map(String::as_str));
        }
        out.push(event);
    }
    Ok(out)
}

/// How many registered, and the guests the page shows by name, from the API
/// behind Luma's event pages. The calendar has only a handful of events, so
/// this is a request each per run. A failure only loses the count.
fn luma_guests(agent: &ureq::Agent, url: &str) -> Option<(u32, Vec<String>)> {
    let slug = url
        .strip_prefix("https://luma.com/")
        .or_else(|| url.strip_prefix("https://lu.ma/"))?;
    if slug.is_empty()
        || !slug
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return None;
    }
    let data: Value =
        serde_json::from_str(&get(agent, &format!("https://api.lu.ma/url?url={slug}")).ok()?)
            .ok()?;
    let count = data["data"]["guest_count"].as_u64()? as u32;
    let names = data["data"]["featured_guests"]
        .as_array()
        .map_or(Vec::new(), |list| {
            list.iter()
                .filter_map(|g| g["name"].as_str().map(String::from))
                .collect()
        });
    Some((count, names))
}

// Philosophia Munich: a public Google Calendar going back to 2020, with weekly
// series in the older years. The feed also lists attendees; only the fields
// read below are ever used.

const PHILOSOPHIA_ICS: &str =
    "https://calendar.google.com/calendar/ical/philosophia.munich%40gmail.com/public/basic.ics";
const PHILOSOPHIA_PAGE: &str = "https://www.philosophiamunich.org/schedule";

/// Google puts HTML in DESCRIPTION.
fn strip_html(s: &str) -> String {
    let s = s
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n");
    let mut out = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn philosophia_events(agent: &ureq::Agent, now: DateTime<Utc>) -> Res<Vec<Event>> {
    let events = ics::parse(&get(agent, PHILOSOPHIA_ICS)?);
    // A moved or edited occurrence of a series comes as its own VEVENT with a
    // RECURRENCE-ID; it replaces the occurrence it names.
    let overridden: HashSet<(String, DateTime<Utc>)> = events
        .iter()
        .filter_map(|e| {
            Some((
                ics::value(e, "UID"),
                ics::time(ics::prop(e, "RECURRENCE-ID"))?,
            ))
        })
        .collect();
    let horizon = now + chrono::Duration::days(366);
    let mut out = Vec::new();
    for e in &events {
        if ics::value(e, "STATUS") == "CANCELLED" {
            continue;
        }
        let (Some(start), end) = (
            ics::time(ics::prop(e, "DTSTART")),
            ics::time(ics::prop(e, "DTEND")),
        ) else {
            continue;
        };
        let length = end.map(|end| end - start);
        let uid = ics::value(e, "UID");
        let is_override = ics::prop(e, "RECURRENCE-ID").is_some();
        let text = strip_html(&ics::value(e, "DESCRIPTION"));
        for t in ics::occurrences(e, horizon) {
            if !is_override && overridden.contains(&(uid.clone(), t)) {
                continue;
            }
            out.push(Event::new(Raw {
                title: &ics::value(e, "SUMMARY"),
                start: t,
                end: length.map(|l| t + l),
                location: &ics::value(e, "LOCATION"),
                online: false,
                text: &text,
                group: "philosophia",
                label: "Philosophia",
                url: PHILOSOPHIA_PAGE,
            }));
        }
    }
    Ok(out)
}

// History nobody publishes in a machine-readable form any more, transcribed
// once. See data/README.md.

#[derive(Deserialize)]
struct Past {
    title: String,
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
    #[serde(default)]
    location: String,
    #[serde(default)]
    excerpt: String,
    groups: Vec<String>,
    label: String,
    url: String,
}

pub fn history() -> Vec<Event> {
    let past: Vec<Past> = serde_json::from_str(include_str!("../data/history.json"))
        .expect("data/history.json is valid");
    past.into_iter()
        .map(|p| {
            let mut e = Event::new(Raw {
                title: &p.title,
                start: p.start,
                end: p.end,
                location: &p.location,
                online: false,
                text: &p.excerpt,
                group: &p.groups[0],
                label: &p.label,
                url: &p.url,
            });
            e.groups = p.groups;
            e
        })
        .collect()
}

// Events that happened but were never announced anywhere we read: organised
// only in the groups' chats. Listed and counted like the rest, marked as
// chat-only since there is nothing to link to. See data/README.md.

#[derive(Deserialize)]
struct Unlisted {
    title: String,
    start: DateTime<Utc>,
    groups: Vec<String>,
}

pub fn unannounced(now: DateTime<Utc>) -> Vec<Event> {
    let listed: Vec<Unlisted> = serde_json::from_str(include_str!("../data/unannounced.json"))
        .expect("data/unannounced.json is valid");
    listed
        .into_iter()
        .filter(|u| u.start < now)
        .map(|u| {
            let mut e = Event::new(Raw {
                title: &u.title,
                start: u.start,
                end: None,
                location: "",
                online: false,
                text: "",
                group: &u.groups[0],
                label: "",
                url: "",
            });
            e.groups = u.groups;
            e.links.clear();
            e.chat = true;
            e
        })
        .collect()
}

// "Who's coming?" polls from the group chats, for events whose pages had no
// RSVPs. See data/README.md.

#[derive(Deserialize)]
struct Poll {
    start: DateTime<Utc>,
    group: String,
    /// Per answer: its votes, and how likely a voter was to come ("50%" is
    /// 0.5, "definitely" 1).
    votes: Vec<(u32, f64)>,
}

impl Poll {
    /// People expected, rounded.
    fn expected(&self) -> u32 {
        self.votes
            .iter()
            .map(|&(n, p)| n as f64 * p)
            .sum::<f64>()
            .round() as u32
    }
}

/// Gives each poll's expected turnout to the group's event that day starting
/// nearest the time the poll named (a dinner poll counts for the meetup day
/// it ends), unless it has more sign-ups already.
pub fn apply_polls(events: &mut [Event]) {
    let polls: Vec<Poll> =
        serde_json::from_str(include_str!("../data/polls.json")).expect("data/polls.json is valid");
    for p in polls {
        if let Some(e) = nearest_on_day(events, p.start, &p.group) {
            e.signups = e.signups.max(p.expected());
        }
    }
}

fn munich_day(t: DateTime<Utc>) -> chrono::NaiveDate {
    t.with_timezone(&chrono_tz::Europe::Berlin).date_naive()
}

/// The group's event that day starting nearest `start`: hand-kept data names
/// an event by its group and time rather than by its title, which feeds
/// change.
fn nearest_on_day<'a>(
    events: &'a mut [Event],
    start: DateTime<Utc>,
    group: &str,
) -> Option<&'a mut Event> {
    events
        .iter_mut()
        .filter(|e| e.groups.iter().any(|g| g == group) && munich_day(e.start) == munich_day(start))
        .min_by_key(|e| (e.start - start).abs())
}

// The organisers' own headcounts. See data/README.md.

#[derive(Deserialize)]
struct Headcount {
    start: DateTime<Utc>,
    group: String,
    came: u32,
    #[serde(default)]
    at_least: bool,
}

/// Organisers counted, so their number beats visitors' reports; a floor
/// ("at least 10") gives way to a report above it.
pub fn apply_headcounts(events: &mut [Event]) {
    let counts: Vec<Headcount> = serde_json::from_str(include_str!("../data/headcounts.json"))
        .expect("data/headcounts.json is valid");
    for h in counts {
        let Some(e) = nearest_on_day(events, h.start, &h.group) else {
            continue;
        };
        let reported_more = e.attended.is_some_and(|c| c.n >= h.came);
        if !(h.at_least && reported_more) {
            e.attended = Some(Came {
                n: h.came,
                at_least: h.at_least,
            });
        }
    }
}

// What the organisers know about a whole series: how many usually came.
// See data/README.md.

#[derive(Deserialize)]
struct Range {
    groups: Vec<String>,
    /// Words every event of the series has in its title.
    title: String,
    /// Events after this are not covered: the series may have grown.
    until: DateTime<Utc>,
    low: u32,
    high: u32,
}

/// Gives each event of a series that nobody counted a headcount within the
/// range the organisers gave: its sign-ups kept inside that range, or the
/// middle of it.
pub fn apply_ranges(events: &mut [Event]) {
    let ranges: Vec<Range> = serde_json::from_str(include_str!("../data/ranges.json"))
        .expect("data/ranges.json is valid");
    for r in ranges {
        let words = title_words(&r.title);
        for e in events.iter_mut().filter(|e| {
            e.attended.is_none()
                && e.start < r.until
                && e.groups == r.groups
                && words.is_subset(&title_words(&e.title))
        }) {
            let n = if e.signed_up() > 0 {
                e.signed_up().clamp(r.low, r.high)
            } else {
                (r.low + r.high).div_ceil(2)
            };
            e.attended = Some(Came { n, at_least: false });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcribed_history_loads_and_is_all_past() {
        let h = history();
        assert!(h.len() >= 20);
        assert!(
            h.iter()
                .all(|e| e.start < Utc::now() && !e.links.is_empty())
        );
    }

    #[test]
    fn unannounced_events_load_unlinked_and_known_groups_only() {
        let u = unannounced(Utc::now());
        assert!(u.len() >= 20);
        assert!(u.iter().all(|e| e.links.is_empty() && e.chat));
        // They survive sanitising despite having no link.
        assert_eq!(crate::event::sanitize(u.clone()).len(), u.len());
        let known = ["acx", "ea", "philosophia"];
        assert!(
            u.iter()
                .all(|e| e.groups.iter().all(|g| known.contains(&g.as_str())))
        );
    }

    #[test]
    fn reading_group_dates_follow_the_semester() {
        use chrono::TimeZone;
        use chrono_tz::Europe::Berlin;
        let at = |y, m, d, h| {
            Berlin
                .with_ymd_and_hms(y, m, d, h, 0, 0)
                .unwrap()
                .with_timezone(&Utc)
        };
        assert_eq!(
            mlphil_time("Fri 17 Jul, 16-17", "Summer 2026"),
            Some((at(2026, 7, 17, 16), at(2026, 7, 17, 17)))
        );
        assert_eq!(
            mlphil_time("Thu 15 Jan, 14-15", "Winter 2025/2026").map(|t| t.0),
            Some(at(2026, 1, 15, 14))
        );
        assert_eq!(
            mlphil_time("Wed 22 Oct, 14-15", "Winter 2025/26").map(|t| t.0),
            Some(at(2025, 10, 22, 14))
        );
        assert!(mlphil_time("tba", "Summer 2026").is_none());
    }

    #[test]
    fn table_cells_come_out_as_plain_text() {
        let html = r#"<tr><th>Date</th></tr><tr><td>Fri 3 Jul, 16-17</td>
            <td><a href="x">Grzejdziak (2026)</a>, "The missing interdiscipline."</td> </td></tr>"#;
        assert_eq!(
            table_cells(html),
            [
                "Fri 3 Jul, 16-17",
                r#"Grzejdziak (2026), "The missing interdiscipline.""#
            ]
        );
    }

    #[test]
    fn a_venue_named_after_its_address_shows_once() {
        let e = serde_json::json!({
            "title": "Dine and Discuss", "eventUrl": "https://www.meetup.com/x/events/1/",
            "dateTime": "2026-09-23T18:00:00+02:00",
            "venue": { "name": "Gabelsbergerstraße 43", "address": "Gabelsbergerstraße 43, 80333 München" },
        });
        let got = meetup_event(&e, "ea", |_| true).unwrap();
        assert_eq!(got.location, "Gabelsbergerstraße 43, 80333 München");
        let e = serde_json::json!({
            "title": "Dinner", "eventUrl": "https://www.meetup.com/x/events/2/",
            "dateTime": "2026-09-23T18:00:00+02:00",
            "venue": { "name": "Bellevue di Monaco", "address": "Müllerstraße 2" },
        });
        assert_eq!(
            meetup_event(&e, "ea", |_| true).unwrap().location,
            "Bellevue di Monaco, Müllerstraße 2"
        );
    }

    #[test]
    fn only_yes_rsvps_count() {
        let rsvps = serde_json::json!([
            { "response": "yes", "name": "Anna" }, { "response": "maybe", "name": "Ben" },
            { "response": "yes", "name": "Cleo" }, { "response": "no", "name": "Dan" }
        ]);
        assert_eq!(forum_yes(&rsvps).len(), 2);
        assert!(forum_yes(&Value::Null).is_empty());
    }

    #[test]
    fn polls_fill_in_only_where_pages_had_fewer() {
        let at = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        let dinner = |start: &str, group: &str, signups: u32| {
            let mut e = Event::new(Raw {
                title: "Community dinner",
                start: at(start),
                end: None,
                location: "",
                online: false,
                text: "",
                group,
                label: "LessWrong",
                url: "https://www.lesswrong.com/events/x",
            });
            e.signups = signups;
            e
        };
        let mut events = vec![
            dinner("2026-04-22T18:30:00+02:00", "acx", 2),
            dinner("2026-04-22T18:30:00+02:00", "ea", 0),
            dinner("2026-05-06T19:00:00+02:00", "acx", 20),
            dinner("2026-04-11T14:00:00+02:00", "acx", 4),
            dinner("2026-04-11T11:00:00+02:00", "acx", 0),
        ];
        apply_polls(&mut events);
        assert_eq!(events[0].signups, 11);
        assert_eq!(events[1].signups, 0, "another group's event");
        assert_eq!(events[2].signups, 20, "the page had more");
        assert_eq!(
            (events[3].signups, events[4].signups),
            (4, 15),
            "the nearest that day"
        );
    }

    #[test]
    fn a_series_range_fills_in_only_uncounted_events() {
        let at = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        let dinner = |title: &str, start: &str, group: &str, signups: u32| {
            let mut e = Event::new(Raw {
                title,
                start: at(start),
                end: None,
                location: "",
                online: false,
                text: "",
                group,
                label: "LessWrong",
                url: "https://www.lesswrong.com/events/x",
            });
            e.signups = signups;
            e
        };
        let mut events = vec![
            dinner(
                "ACX community dinner",
                "2026-04-22T18:30:00+02:00",
                "acx",
                11,
            ),
            dinner("Community Dinner", "2026-08-11T18:30:00+02:00", "acx", 2),
            dinner(
                "ACX community dinner",
                "2026-09-09T18:30:00+02:00",
                "acx",
                0,
            ),
            dinner(
                "ACX community dinner",
                "2026-07-08T18:30:00+02:00",
                "acx",
                4,
            ),
            dinner("EA community dinner", "2026-08-26T18:30:00+02:00", "ea", 5),
            dinner(
                "Dinner with a visiting rationalist",
                "2026-04-19T17:00:00+02:00",
                "acx",
                0,
            ),
            dinner(
                "ACX community dinner",
                "2026-10-07T18:30:00+02:00",
                "acx",
                20,
            ),
        ];
        events[3].attended = Some(Came {
            n: 3,
            at_least: false,
        });
        apply_ranges(&mut events);
        let n: Vec<Option<u32>> = events.iter().map(|e| e.attended.map(|c| c.n)).collect();
        assert_eq!(n, [Some(11), Some(5), Some(9), Some(3), None, None, None]);
    }

    #[test]
    fn organisers_counts_beat_reports_but_floors_give_way() {
        let at = |s: &str| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        let game = |start: &str, reported: Option<u32>| {
            let mut e = Event::new(Raw {
                title: "The Estimation Game",
                start: at(start),
                end: None,
                location: "",
                online: false,
                text: "",
                group: "acx",
                label: "LessWrong",
                url: "https://www.lesswrong.com/events/x",
            });
            e.attended = reported.map(|n| Came { n, at_least: false });
            e
        };
        // 11 April: counted 30; 29 July and 30 August: at least 10.
        let mut events = vec![
            game("2026-04-11T15:30:00+02:00", Some(12)),
            game("2026-07-29T18:30:00+02:00", Some(14)),
            game("2026-08-30T14:00:00+02:00", None),
        ];
        apply_headcounts(&mut events);
        let got: Vec<Option<Came>> = events.iter().map(|e| e.attended).collect();
        assert_eq!(
            got[0],
            Some(Came {
                n: 30,
                at_least: false
            })
        );
        assert_eq!(
            got[1],
            Some(Came {
                n: 14,
                at_least: false
            })
        );
        assert_eq!(
            got[2],
            Some(Came {
                n: 10,
                at_least: true
            })
        );
    }

    #[test]
    fn a_fifty_percent_vote_is_half_a_person() {
        let poll: Poll = serde_json::from_str(
            r#"{ "start": "2026-06-23T18:30:00+02:00", "group": "acx",
                 "votes": [[4, 0.9], [2, 0.7], [1, 0.5], [1, 0.3], [1, 0.1]] }"#,
        )
        .unwrap();
        assert_eq!(poll.expected(), 6);
    }

    #[test]
    fn google_descriptions_lose_their_html() {
        assert_eq!(
            strip_html("Read <b>ch. 1</b><br>Bring snacks &amp; drinks"),
            "Read ch. 1\nBring snacks & drinks"
        );
    }
}
