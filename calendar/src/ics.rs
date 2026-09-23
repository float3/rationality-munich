//! Reading the iCalendar feeds Meetup, Luma and Google Calendar publish, and
//! writing our own.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc, Weekday};
use chrono_tz::{Europe::Berlin, Tz};

use crate::event::Event;

pub struct Prop {
    pub value: String,
    pub tzid: Option<String>,
}

/// Property name to every value it had; EXDATE may repeat.
pub type IcsEvent = HashMap<String, Vec<Prop>>;

pub fn parse(text: &str) -> Vec<IcsEvent> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.lines() {
        match (
            raw.strip_prefix(' ').or_else(|| raw.strip_prefix('\t')),
            lines.last_mut(),
        ) {
            (Some(cont), Some(last)) => last.push_str(cont),
            _ => lines.push(raw.to_string()),
        }
    }
    let mut events = Vec::new();
    let mut cur: Option<IcsEvent> = None;
    // VALARM and friends nest inside VEVENT; their properties are not the event's.
    let mut depth = 0;
    for line in lines {
        if line == "BEGIN:VEVENT" {
            cur = Some(IcsEvent::new());
            depth = 0;
        } else if line == "END:VEVENT" {
            events.extend(cur.take());
        } else if line.starts_with("BEGIN:") {
            depth += 1;
        } else if line.starts_with("END:") {
            depth -= 1;
        } else if let (Some(ev), Some((head, value)), 0) =
            (cur.as_mut(), line.split_once(':'), depth)
        {
            let mut parts = head.split(';');
            let name = parts.next().unwrap_or("").to_uppercase();
            let tzid = parts
                .find_map(|p| p.strip_prefix("TZID="))
                .map(|t| t.trim_matches('"').to_string());
            ev.entry(name).or_default().push(Prop {
                value: value.to_string(),
                tzid,
            });
        }
    }
    events
}

pub fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

pub fn prop<'a>(ev: &'a IcsEvent, name: &str) -> Option<&'a Prop> {
    ev.get(name).and_then(|v| v.first())
}

pub fn value(ev: &IcsEvent, name: &str) -> String {
    prop(ev, name)
        .map(|p| unescape(&p.value))
        .unwrap_or_default()
}

fn zone(tzid: Option<&str>) -> Tz {
    tzid.and_then(|t| t.parse().ok()).unwrap_or(Berlin)
}

/// A single DATE or DATE-TIME value; floating times are read as Munich time.
fn parse_time(v: &str, tzid: Option<&str>) -> Option<DateTime<Utc>> {
    let naive = if v.len() == 8 {
        NaiveDate::parse_from_str(v, "%Y%m%d")
            .ok()?
            .and_hms_opt(0, 0, 0)?
    } else if let Some(utc) = v.strip_suffix('Z') {
        return Some(
            NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S")
                .ok()?
                .and_utc(),
        );
    } else {
        NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?
    };
    zone(tzid)
        .from_local_datetime(&naive)
        .earliest()
        .map(|t| t.with_timezone(&Utc))
}

pub fn time(p: Option<&Prop>) -> Option<DateTime<Utc>> {
    let p = p?;
    parse_time(&p.value, p.tzid.as_deref())
}

/// Every start time a recurring event has, up to `horizon`.
///
/// Only weekly rules are expanded, which is all the feeds we read use; any
/// other rule yields just the first occurrence. Occurrences listed in EXDATE
/// are dropped. Times step in local time, so 19:00 stays 19:00 across DST.
pub fn occurrences(ev: &IcsEvent, horizon: DateTime<Utc>) -> Vec<DateTime<Utc>> {
    let Some(dtstart) = prop(ev, "DTSTART") else {
        return vec![];
    };
    let Some(first) = time(Some(dtstart)) else {
        return vec![];
    };
    let Some(rule) = prop(ev, "RRULE") else {
        return vec![first];
    };
    let parts: HashMap<&str, &str> = rule
        .value
        .split(';')
        .filter_map(|p| p.split_once('='))
        .collect();
    if parts.get("FREQ") != Some(&"WEEKLY") {
        return vec![first];
    }
    let tz = if dtstart.value.ends_with('Z') {
        None
    } else {
        Some(zone(dtstart.tzid.as_deref()))
    };
    let local = |t: DateTime<Utc>| match tz {
        Some(tz) => t.with_timezone(&tz).naive_local(),
        None => t.naive_utc(),
    };
    let to_utc = |n: NaiveDateTime| match tz {
        Some(tz) => tz
            .from_local_datetime(&n)
            .earliest()
            .map(|t| t.with_timezone(&Utc)),
        None => Some(n.and_utc()),
    };
    let interval: i64 = parts
        .get("INTERVAL")
        .and_then(|i| i.parse().ok())
        .unwrap_or(1)
        .max(1);
    let count: Option<usize> = parts.get("COUNT").and_then(|c| c.parse().ok());
    // UNTIL is UTC (or a date); compare it in UTC.
    let until = parts.get("UNTIL").and_then(|u| parse_time(u, None));
    let start_local = local(first);
    let mut days: Vec<Weekday> = parts
        .get("BYDAY")
        .map(|d| d.split(',').filter_map(weekday).collect())
        .unwrap_or_default();
    if days.is_empty() {
        days.push(start_local.weekday());
    }
    days.sort_by_key(|d| d.num_days_from_monday());

    let excluded: Vec<DateTime<Utc>> = ev
        .get("EXDATE")
        .into_iter()
        .flatten()
        .flat_map(|p| {
            p.value
                .split(',')
                .filter_map(|v| parse_time(v, p.tzid.as_deref()))
                .collect::<Vec<_>>()
        })
        .collect();

    let monday =
        start_local.date() - Duration::days(start_local.weekday().num_days_from_monday() as i64);
    let mut out = Vec::new();
    let mut produced = 0;
    'weeks: for week in (0..).step_by(interval as usize) {
        for day in &days {
            let date = monday + Duration::days(week * 7 + day.num_days_from_monday() as i64);
            let Some(t) = to_utc(date.and_time(start_local.time())) else {
                continue;
            };
            if t < first {
                continue;
            }
            // DTSTART is always the first occurrence, even past UNTIL: Google
            // leaves such rules behind when a series is split.
            let past_end = until.is_some_and(|u| t > u) || t > horizon;
            if t != first && (past_end || count.is_some_and(|c| produced >= c)) {
                break 'weeks;
            }
            produced += 1;
            if !excluded.contains(&t) {
                out.push(t);
            }
            if out.len() >= 1000 {
                break 'weeks;
            }
        }
    }
    out
}

fn weekday(code: &str) -> Option<Weekday> {
    // BYDAY may carry an ordinal (`-1SU`); weekly rules do not use one.
    Some(
        match code.trim_start_matches(|c: char| c == '-' || c == '+' || c.is_ascii_digit()) {
            "MO" => Weekday::Mon,
            "TU" => Weekday::Tue,
            "WE" => Weekday::Wed,
            "TH" => Weekday::Thu,
            "FR" => Weekday::Fri,
            "SA" => Weekday::Sat,
            "SU" => Weekday::Sun,
            _ => return None,
        },
    )
}

// Writing

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

/// Lines may be at most 75 octets; continuation lines start with a space.
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut len = 0;
    for c in line.chars() {
        if len + c.len_utf8() > 75 {
            out.push_str("\r\n ");
            len = 1;
        }
        out.push(c);
        len += c.len_utf8();
    }
    out
}

pub fn utc(t: DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

/// The text calendar apps show under the event: the excerpt, then every
/// announcement and the permalink.
pub fn description(e: &Event, permalink: &str) -> String {
    let mut links: Vec<String> = e.links.iter().map(|(l, u)| format!("{l}: {u}")).collect();
    links.push(format!("rationality-munich.com: {permalink}"));
    [e.excerpt.clone(), links.join("\n")]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn calendar(name: &str, events: &[&Event], now: DateTime<Utc>, base: &str) -> String {
    let mut lines: Vec<String> = vec![
        "BEGIN:VCALENDAR".into(),
        "VERSION:2.0".into(),
        "PRODID:-//rationality-munich.com//calendar//EN".into(),
        "CALSCALE:GREGORIAN".into(),
        "METHOD:PUBLISH".into(),
        format!("X-WR-CALNAME:{}", escape(name)),
        "X-WR-TIMEZONE:Europe/Berlin".into(),
        "X-WR-CALDESC:Events from the rationality\\, EA and AI safety groups in Munich".into(),
        "REFRESH-INTERVAL;VALUE=DURATION:PT1H".into(),
        "X-PUBLISHED-TTL:PT1H".into(),
    ];
    for e in events {
        let permalink = format!("{base}#{}", e.id);
        lines.push("BEGIN:VEVENT".into());
        // The id is what a subscribed app matches on to update an event.
        lines.push(format!("UID:{}@rationality-munich.com", e.id));
        lines.push(format!("DTSTAMP:{}", utc(now)));
        lines.push(format!("DTSTART:{}", utc(e.start)));
        lines.push(format!("DTEND:{}", utc(e.end_or_default())));
        lines.push(format!("SUMMARY:{}", escape(&e.title)));
        lines.push(format!(
            "DESCRIPTION:{}",
            escape(&description(e, &permalink))
        ));
        lines.push(format!("URL:{permalink}"));
        if !e.place().is_empty() {
            lines.push(format!("LOCATION:{}", escape(e.place())));
        }
        lines.push("END:VEVENT".into());
    }
    lines.push("END:VCALENDAR".into());
    lines
        .iter()
        .map(|l| fold(l))
        .collect::<Vec<_>>()
        .join("\r\n")
        + "\r\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> IcsEvent {
        parse(&format!(
            "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\n{text}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
        ))
        .pop()
        .unwrap()
    }

    fn berlin(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        Berlin
            .with_ymd_and_hms(y, m, d, h, min, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn lines_fold_at_75_octets_on_char_boundaries() {
        let line = format!("DESCRIPTION:{}", "München ".repeat(30));
        let folded = fold(&line);
        assert!(folded.split("\r\n").all(|l| l.len() <= 75));
        assert_eq!(folded.replace("\r\n ", ""), line);
    }

    #[test]
    fn times_respect_tzid() {
        let e = one("DTSTART;TZID=Europe/Berlin:20260921T183000");
        assert_eq!(time(prop(&e, "DTSTART")), Some(berlin(2026, 9, 21, 18, 30)));
    }

    #[test]
    fn alarms_inside_an_event_do_not_overwrite_it() {
        let e = one(
            "SUMMARY:Reading\r\nBEGIN:VALARM\r\nDESCRIPTION:Reminder\r\nEND:VALARM\r\nDESCRIPTION:Real",
        );
        assert_eq!(value(&e, "DESCRIPTION"), "Real");
    }

    #[test]
    fn weekly_rules_expand_across_dst_and_skip_exdates() {
        let e = one("DTSTART;TZID=Europe/Berlin:20241015T190000\r\n\
             RRULE:FREQ=WEEKLY;WKST=MO;UNTIL=20241112T225959Z;BYDAY=TU\r\n\
             EXDATE;TZID=Europe/Berlin:20241029T190000");
        let got = occurrences(&e, Utc::now());
        assert_eq!(
            got,
            [
                berlin(2024, 10, 15, 19, 0),
                berlin(2024, 10, 22, 19, 0),
                // 29 Oct is excluded; the clocks changed on the 27th.
                berlin(2024, 11, 5, 19, 0),
                berlin(2024, 11, 12, 19, 0),
            ]
        );
    }

    #[test]
    fn fortnightly_rules_with_a_count() {
        let e = one("DTSTART:20200106T180000Z\r\nRRULE:FREQ=WEEKLY;INTERVAL=2;COUNT=3");
        let got = occurrences(&e, Utc::now());
        assert_eq!(got.len(), 3);
        assert_eq!(got[2] - got[0], Duration::weeks(4));
    }

    #[test]
    fn the_start_counts_even_when_until_is_earlier() {
        let e = one("DTSTART;TZID=Europe/Berlin:20240430T190000\r\n\
             RRULE:FREQ=WEEKLY;WKST=MO;UNTIL=20240130T225959Z;BYDAY=TU");
        assert_eq!(occurrences(&e, Utc::now()), [berlin(2024, 4, 30, 19, 0)]);
    }

    #[test]
    fn other_rules_yield_the_first_occurrence_only() {
        let e = one("DTSTART:20200106T180000Z\r\nRRULE:FREQ=MONTHLY;COUNT=5");
        assert_eq!(occurrences(&e, Utc::now()).len(), 1);
    }
}
