//! Takes "about how many came?" reports for past events on
//! rationality-munich.com/calendar/past/ and keeps them in a JSON file that
//! the calendar reads on its next run.
//!
//! A report stores the event, the number and the time; nothing about who sent
//! it. The client address (nginx's X-Real-IP) only ever lives in memory, to
//! limit how often one address can report, and is forgotten within a day.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tiny_http::{Header, Method, Request, Response, Server};

#[derive(Clone, Serialize, Deserialize)]
struct Report {
    n: u32,
    at: DateTime<Utc>,
}

type Reports = HashMap<String, Vec<Report>>;

const MAX_BODY: usize = 256;
const MAX_ATTENDEES: u32 = 1000;
/// Reports one address may send per hour.
const PER_HOUR: usize = 20;
/// Reports kept per event; enough for a median, too few to flood.
const PER_EVENT: usize = 100;

struct State {
    file: PathBuf,
    known: PathBuf,
    reports: Reports,
    /// Per address: when its hour started, and how many reports since.
    recent: HashMap<String, (Instant, usize)>,
    /// Hashes of address + event, so one address reports an event once a day.
    seen: HashSet<u64>,
    day: NaiveDate,
}

/// FNV-1a: only ever compared within this process's memory.
fn hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 120
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// `id=...&n=...`. Neither needs percent-decoding: ids are `[a-z0-9-]`.
fn parse(body: &str) -> Option<(String, u32)> {
    let mut id = None;
    let mut n = None;
    for pair in body.split('&') {
        match pair.split_once('=')? {
            ("id", v) if valid_id(v) => id = Some(v.to_string()),
            ("n", v) => n = v.parse::<u32>().ok(),
            _ => {}
        }
    }
    let n = n.filter(|n| (1..=MAX_ATTENDEES).contains(n))?;
    Some((id?, n))
}

/// Ids of past events, written by the calendar on every run.
fn is_past_event(known: &Path, id: &str) -> bool {
    fs::read_to_string(known)
        .ok()
        .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
        .is_some_and(|ids| ids.iter().any(|k| k == id))
}

fn save(file: &Path, reports: &Reports) -> std::io::Result<()> {
    let tmp = file.with_extension("tmp");
    fs::write(&tmp, serde_json::to_string(reports)?)?;
    fs::rename(&tmp, file)
}

fn reply(status: u16, message: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let body = serde_json::json!({ "ok": status == 200, "message": message }).to_string();
    Response::from_string(body)
        .with_status_code(status)
        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap())
}

fn handle(state: &mut State, req: &mut Request) -> Response<std::io::Cursor<Vec<u8>>> {
    if req.url() != "/calendar/attend" {
        return reply(404, "Not found.");
    }
    if *req.method() != Method::Post {
        return reply(405, "Send a POST.");
    }
    let mut body = String::new();
    if req
        .as_reader()
        .take(MAX_BODY as u64 + 1)
        .read_to_string(&mut body)
        .is_err()
        || body.len() > MAX_BODY
    {
        return reply(413, "Too long.");
    }
    let Some((id, n)) = parse(&body) else {
        return reply(400, "Send a number between 1 and 1000 for a past event.");
    };
    if !is_past_event(&state.known, &id) {
        return reply(404, "No such past event.");
    }

    let address = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("X-Real-IP"))
        .map(|h| h.value.to_string())
        .or_else(|| req.remote_addr().map(|a| a.ip().to_string()))
        .unwrap_or_default();
    let today = Utc::now().date_naive();
    if today != state.day {
        state.day = today;
        state.seen.clear();
        state.recent.clear();
    }
    let (since, count) = state
        .recent
        .entry(address.clone())
        .or_insert((Instant::now(), 0));
    if since.elapsed() > Duration::from_secs(3600) {
        *since = Instant::now();
        *count = 0;
    }
    if *count >= PER_HOUR {
        return reply(429, "That's a lot of reports; try again in an hour.");
    }
    if !state.seen.insert(hash(&format!("{address} {id}"))) {
        return reply(409, "Already counted, thanks!");
    }
    let list = state.reports.entry(id).or_default();
    if list.len() >= PER_EVENT {
        return reply(429, "This event has enough reports, thanks!");
    }
    *count += 1;
    list.push(Report { n, at: Utc::now() });
    if let Err(err) = save(&state.file, &state.reports) {
        eprintln!("saving reports: {err}");
        return reply(500, "Could not save; please try later.");
    }
    reply(200, "Thanks! It shows on the page within the hour.")
}

fn main() {
    let dir = PathBuf::from(std::env::var("STATE_DIRECTORY").unwrap_or_else(|_| "out".into()));
    let file = dir.join("attendance.json");
    let addr = std::env::var("ATTENDANCE_ADDR").unwrap_or_else(|_| "127.0.0.1:8098".into());
    let known =
        PathBuf::from(std::env::var("KNOWN_EVENTS").unwrap_or_else(|_| "out/past-ids.json".into()));
    let reports = fs::read_to_string(&file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut state = State {
        file,
        known,
        reports,
        recent: HashMap::new(),
        seen: HashSet::new(),
        day: Utc::now().date_naive(),
    };
    let server = Server::http(&addr).expect("listen");
    eprintln!("listening on {addr}");
    for mut req in server.incoming_requests() {
        let response = handle(&mut state, &mut req);
        let _ = req.respond(response);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_well_formed_reports_parse() {
        assert_eq!(
            parse("id=2026-09-09-acx-community-dinner&n=12"),
            Some(("2026-09-09-acx-community-dinner".into(), 12))
        );
        assert_eq!(parse("n=3&id=x-1"), Some(("x-1".into(), 3)));
        assert_eq!(parse("id=x&n=0"), None);
        assert_eq!(parse("id=x&n=1001"), None);
        assert_eq!(parse("id=../etc&n=3"), None);
        assert_eq!(parse("id=X<script>&n=3"), None);
        assert_eq!(parse("n=3"), None);
    }
}
