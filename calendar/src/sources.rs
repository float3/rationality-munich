//! Where events come from. Each source returns everything it knows, past and
//! upcoming; the caller decides what goes where.

use std::collections::HashSet;
use std::error::Error;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::event::{Event, Raw, web_url};
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
        Source {
            key: "meetup",
            label: "Meetup",
            fetch: Box::new(|agent, _| meetup_events(agent)),
        },
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
    ]
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

// Meetup: an iCalendar feed of upcoming events only. Past ones stay on the
// page through the archive.

const MEETUP_ICS: &str = "https://www.meetup.com/effective-altruism-munich/events/ical/";

/// Meetup's feed has no location; the event page's JSON-LD does.
fn meetup_venue(agent: &ureq::Agent, url: &str) -> Res<(String, bool)> {
    let page = get(agent, url)?;
    let mut rest = page.as_str();
    while let Some(i) = rest.find("<script type=\"application/ld+json\"") {
        rest = &rest[i..];
        let Some(open) = rest.find('>') else { break };
        let Some(close) = rest.find("</script>") else {
            break;
        };
        let block = &rest[open + 1..close.max(open + 1)];
        rest = &rest[close.max(open + 1)..];
        let Ok(data) = serde_json::from_str::<Value>(block) else {
            continue;
        };
        let items = match data {
            Value::Array(items) => items,
            one => vec![one],
        };
        if let Some(item) = items.iter().find(|i| i["@type"] == "Event") {
            let loc = &item["location"];
            let mode = item["eventAttendanceMode"].as_str().unwrap_or("");
            if loc["@type"] == "VirtualLocation" || mode.contains("Online") {
                return Ok((String::new(), true));
            }
            let mut parts: Vec<String> = Vec::new();
            let name = loc["name"].as_str().unwrap_or("");
            let street = loc["address"]["streetAddress"].as_str().unwrap_or("");
            for p in std::iter::once(name)
                .chain(street.split(','))
                .map(str::trim)
            {
                if !p.is_empty()
                    && !["Germany", "BY", "de"].contains(&p)
                    && !parts.iter().any(|q| q.contains(p))
                {
                    parts.push(p.to_string());
                }
            }
            return Ok((parts.join(", "), false));
        }
    }
    Ok((String::new(), false))
}

fn meetup_events(agent: &ureq::Agent) -> Res<Vec<Event>> {
    let mut out = Vec::new();
    for e in ics::parse(&get(agent, MEETUP_ICS)?) {
        let Some(start) = ics::time(ics::prop(&e, "DTSTART")) else {
            continue;
        };
        let url = ics::value(&e, "URL");
        let text = ics::value(&e, "DESCRIPTION");
        let text = text
            .strip_prefix("Effective Altruism Munich\n")
            .unwrap_or(&text);
        // Without the venue the event still shows, so a failure here is only logged.
        // The URL comes from the feed, so it is only fetched when it is a Meetup page.
        let venue = if url.starts_with("https://www.meetup.com/") && web_url(&url) {
            meetup_venue(agent, &url)
        } else {
            Err("not a meetup.com page".into())
        };
        let (location, online) = venue.unwrap_or_else(|err| {
            eprintln!("meetup venue {url:?}: {err}");
            (String::new(), false)
        });
        out.push(Event::new(Raw {
            title: &ics::value(&e, "SUMMARY"),
            start,
            end: ics::time(ics::prop(&e, "DTEND")),
            location: &location,
            online,
            text,
            group: "ea",
            label: "Meetup",
            url: &url,
        }));
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
    let past: Vec<Past> = serde_json::from_str(include_str!("../data/acx-substack.json"))
        .expect("data/acx-substack.json is valid");
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

// Events that happened but were never announced anywhere we read. They count
// in the statistics only: with nothing to link to, they are not listed.

/// The fortnightly community dinner: every second Wednesday at 18:30 since
/// 22 April 2026, almost without a break. A date is skipped when a dinner was
/// announced that week (it is counted already), which also covers the weeks
/// EA Munich's monthly community dinner takes its place.
fn community_dinners(now: DateTime<Utc>, announced: &[Event]) -> Vec<Event> {
    use chrono::{Datelike, TimeZone};
    use chrono_tz::Europe::Berlin;
    let week = |t: DateTime<Utc>| t.with_timezone(&Berlin).iso_week();
    let dinners: Vec<DateTime<Utc>> = announced
        .iter()
        .filter(|e| e.title.to_lowercase().contains("dinner"))
        .map(|e| e.start)
        .collect();
    let mut out = Vec::new();
    let mut t = Berlin
        .with_ymd_and_hms(2026, 4, 22, 18, 30, 0)
        .unwrap()
        .with_timezone(&Utc);
    while t + chrono::Duration::hours(3) < now {
        if !dinners.iter().any(|d| week(*d) == week(t)) {
            out.push(unlisted("Community dinner", t, "acx"));
        }
        // Step in local time so 18:30 stays 18:30 across DST.
        let next = t.with_timezone(&Berlin).naive_local() + chrono::Duration::days(14);
        t = Berlin
            .from_local_datetime(&next)
            .earliest()
            .unwrap()
            .with_timezone(&Utc);
    }
    out
}

fn unlisted(title: &str, start: DateTime<Utc>, group: &str) -> Event {
    let mut e = Event::new(Raw {
        title,
        start,
        end: None,
        location: "",
        online: false,
        text: "",
        group,
        label: "",
        url: "",
    });
    e.links.clear();
    e
}

#[derive(Deserialize)]
struct Unlisted {
    title: String,
    start: DateTime<Utc>,
    groups: Vec<String>,
}

/// Past events counted in the statistics but never announced: the community
/// dinners, and whatever data/unannounced.json lists.
pub fn unannounced(now: DateTime<Utc>, announced: &[Event]) -> Vec<Event> {
    let listed: Vec<Unlisted> = serde_json::from_str(include_str!("../data/unannounced.json"))
        .expect("data/unannounced.json is valid");
    let mut out = community_dinners(now, announced);
    for u in listed.into_iter().filter(|u| u.start < now) {
        let mut e = unlisted(&u.title, u.start, &u.groups[0]);
        e.groups = u.groups;
        out.push(e);
    }
    out
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
    fn dinners_every_second_wednesday_except_weeks_with_an_announced_dinner() {
        use chrono::TimeZone;
        use chrono_tz::Europe::Berlin;
        let at = |m, d, h| {
            Berlin
                .with_ymd_and_hms(2026, m, d, h, 30, 0)
                .unwrap()
                .with_timezone(&Utc)
        };
        let announced = |title: &str, t| {
            let mut e = crate::event::tests::ev(title, 0, "X");
            e.start = t;
            e
        };
        // Only a dinner announced in the same week skips a date; other
        // announced events (board games on 6 May) do not.
        let got = community_dinners(
            at(6, 20, 12),
            &[
                announced("EA community dinner", at(6, 2, 18)),
                announced("Board games", at(5, 6, 18)),
            ],
        );
        let days: Vec<u32> = got
            .iter()
            .map(|e| {
                use chrono::Datelike;
                e.start.with_timezone(&Berlin).day()
            })
            .collect();
        // 22 Apr, 6 May, 20 May, (3 Jun skipped: dinner announced on 2 Jun), 17 Jun
        assert_eq!(days, [22, 6, 20, 17]);
        assert!(got.iter().all(|e| {
            e.start
                .with_timezone(&Berlin)
                .format("%a %H:%M")
                .to_string()
                == "Wed 18:30"
        }));
        assert!(
            got.iter()
                .all(|e| e.links.is_empty() && e.groups == ["acx"])
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
