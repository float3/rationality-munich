//! Where events come from. Each source returns everything it knows, past and
//! upcoming; the caller decides what goes where.

use std::collections::HashSet;
use std::error::Error;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::event::{Event, Raw};
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
              contents { plaintextDescription } } } }
"#;

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
            out.push(Event::new(Raw {
                title: p["title"].as_str().unwrap_or(""),
                start,
                end: forum_time(&p["endTime"]),
                location: p["location"].as_str().unwrap_or(""),
                online: p["onlineEvent"].as_bool().unwrap_or(false),
                text: p["contents"]["plaintextDescription"].as_str().unwrap_or(""),
                group,
                label,
                url: p["pageUrl"].as_str().unwrap_or(""),
            }));
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
                     venue { name address } } }
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
        if !x.is_empty() && !place.contains(&x) && x != "Online event" {
            place.push(x);
        }
    }
    // Descriptions are Markdown; the excerpt wants plain text.
    let text = e["description"]
        .as_str()
        .unwrap_or("")
        .replace(['*', '#', '_'], "");
    Some(Event::new(Raw {
        title,
        start,
        end: forum_time(&e["endTime"]),
        location: &place.join(", "),
        online: e["isOnline"].as_bool().unwrap_or(false),
        text: &text,
        group,
        label: "Meetup",
        url,
    }))
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
        out.push(Event::new(Raw {
            title: &ics::value(&e, "SUMMARY"),
            start,
            end: ics::time(ics::prop(&e, "DTEND")),
            location: if online { "" } else { &location },
            online,
            text: "",
            group: "ea",
            label: "Luma",
            url: &url,
        }));
    }
    Ok(out)
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
    fn google_descriptions_lose_their_html() {
        assert_eq!(
            strip_html("Read <b>ch. 1</b><br>Bring snacks &amp; drinks"),
            "Read ch. 1\nBring snacks & drinks"
        );
    }
}
