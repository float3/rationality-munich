//! One event, however many places announced it, and the rules for merging
//! cross-posts and cleaning up what the feeds send.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use chrono_tz::Europe::Berlin;
use serde::{Deserialize, Serialize};

pub const EXCERPT_CHARS: usize = 240;

/// The communities people can switch on and off. Keys are part of feed URLs
/// (`feeds/acx+ea.ics`), so renaming one breaks existing subscriptions.
pub const GROUPS: &[(&str, &str)] = &[
    ("acx", "LW/ACX Munich"),
    ("ea", "EA Munich"),
    ("philosophia", "Philosophia Munich"),
    // Further Munich groups, from recthink.substack.com/p/germany. Off by
    // default, one feed each.
    ("mlphil", "Philosophy of ML reading group"),
    ("geb", "Gödel, Escher, Bach reading group"),
    ("agi", "AGI Munich"),
    ("skeptics", "Skeptics in the Pub"),
    ("science", "Science Club Munich"),
    ("minds", "Minds in Motion"),
    ("curious", "Lifelong Curious & Book Lovers"),
    ("silentbooks", "Silent Book Club"),
    ("culture", "Culture Club Munich"),
];

/// Groups with a feed for every combination of them (`feeds/acx+ea.ics`).
/// The rest get one feed each, or 2^n files would pile up.
pub const COMBO_GROUPS: &[&str] = &["acx", "ea", "philosophia"];

/// On until the visitor picks otherwise. Philosophia is its own community
/// rather than part of the rationality/EA scene, so it starts off.
pub const DEFAULT_GROUPS: &[&str] = &["acx", "ea"];

pub fn group_name(key: &str) -> &str {
    GROUPS
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(key, |(_, name)| name)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    /// Permalink anchor and file name, e.g. `2026-09-26-petrov-day-ritual`.
    /// Assigned after merging; not kept in caches.
    #[serde(skip)]
    pub id: String,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub location: String,
    pub online: bool,
    pub excerpt: String,
    /// Keys from `GROUPS`.
    pub groups: Vec<String>,
    /// (source label, url); the first one is the event's canonical link.
    pub links: Vec<(String, String)>,
    /// Organised only in a group chat: nothing to link to. See
    /// `sources::unannounced`.
    #[serde(default)]
    pub chat: bool,
    /// Median of visitors' "about how many came" reports, and how many there
    /// are. Read fresh every run; not kept in caches.
    #[serde(skip)]
    pub attended: Option<(u32, usize)>,
    /// How many said they would come: a page's RSVP count or a chat poll's
    /// yes votes. Only counts are read, never who. Cross-posts keep the
    /// largest, since the same people often sign up on several sites.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub signups: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

pub struct Raw<'a> {
    pub title: &'a str,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
    pub location: &'a str,
    pub online: bool,
    pub text: &'a str,
    pub group: &'a str,
    pub label: &'a str,
    pub url: &'a str,
}

impl Event {
    pub fn new(r: Raw) -> Self {
        let title = r.title.trim().to_string();
        Event {
            id: String::new(),
            excerpt: excerpt(&drop_title(r.text, &title)),
            title,
            start: r.start,
            end: r.end,
            location: r.location.trim().to_string(),
            online: r.online,
            groups: vec![r.group.to_string()],
            links: vec![(r.label.to_string(), r.url.to_string())],
            chat: false,
            attended: None,
            signups: 0,
        }
    }

    /// Reported attendance if anyone reported it, else the sign-ups.
    pub fn headcount(&self) -> Option<u32> {
        self.attended
            .map(|(n, _)| n)
            .or(Some(self.signups).filter(|&n| n > 0))
    }

    pub fn place(&self) -> &str {
        if self.online && self.location.is_empty() {
            "Online"
        } else {
            &self.location
        }
    }

    /// Calendar apps need an end; announcements often leave it out.
    pub fn end_or_default(&self) -> DateTime<Utc> {
        self.end.unwrap_or(self.start + Duration::hours(3))
    }

    pub fn in_any(&self, keys: &[&str]) -> bool {
        self.groups.iter().any(|g| keys.contains(&g.as_str()))
    }
}

// Text helpers

/// Feed text ends up on one line of the page and of the ICS file, so control
/// characters (a bare CR or LF would start a new ICS content line) are dropped.
fn one_line(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .to_string()
}

/// Links are only ever web links; anything else a feed supplies (`javascript:`,
/// a URL carrying a line break) is not published.
pub fn web_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// Caches written before groups became keys hold display names.
fn group_key(g: &str) -> Option<String> {
    GROUPS
        .iter()
        .find(|(k, name)| *k == g || *name == g)
        .map(|(k, _)| k.to_string())
}

/// Applied to fetched, cached and archived events alike, before anything is
/// rendered.
pub fn sanitize(events: Vec<Event>) -> Vec<Event> {
    events
        .into_iter()
        .filter_map(|mut e| {
            e.title = one_line(&e.title);
            e.location = one_line(
                e.location
                    .trim_end_matches(", Germany")
                    .trim_end_matches(", Deutschland"),
            );
            e.links.retain(|(_, url)| web_url(url));
            e.groups = e.groups.iter().filter_map(|g| group_key(g)).collect();
            e.groups
                .sort_by_key(|g| GROUPS.iter().position(|(k, _)| k == g));
            e.groups.dedup();
            // Only chat events may lack a link.
            ((e.chat || !e.links.is_empty()) && !e.groups.is_empty() && !e.title.is_empty())
                .then_some(e)
        })
        .collect()
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn excerpt(text: &str) -> String {
    let text = collapse_ws(text);
    if text.chars().count() <= EXCERPT_CHARS {
        return text;
    }
    let cut: String = text.chars().take(EXCERPT_CHARS).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{} …", cut.trim_end_matches([',', '.', ';', ':', '–', '-']))
}

/// Announcements often open by repeating the title; the card already shows it.
fn drop_title(text: &str, title: &str) -> String {
    let title = title.to_lowercase();
    let lines: Vec<&str> = text.trim().lines().collect();
    let skip = lines
        .iter()
        .take_while(|l| l.to_lowercase().contains(&title))
        .count();
    lines[skip..].join("\n")
}

// Merging cross-posts

const STOPWORDS: &[&str] = &[
    "a", "an", "and", "at", "for", "in", "of", "on", "the", "to", "with", "munich", "münchen",
    "ea", "acx", "lw", "meetup", "meetups", "event", "events",
];

fn title_words(title: &str) -> HashSet<String> {
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !STOPWORDS.contains(w))
        .map(|w| {
            if w.len() > 3 {
                w.trim_end_matches('s').to_string()
            } else {
                w.to_string()
            }
        })
        .collect()
}

const TWIN_GAP: i64 = 30;

/// Cross-posts start at about the same time and share most of the words in
/// their titles. The same event is often titled differently per platform
/// ("Petrov Day" vs "Petrov Day Ritual: Munich"), so the shorter title's words
/// only need to mostly appear in the longer one.
///
/// A time moved after posting ("picnic moved to 20:00 because of the heat")
/// leaves the copies hours apart; on the same day, fully matching titles are
/// still the same event.
pub fn same_event(a: &Event, b: &Event) -> bool {
    let (wa, wb) = (title_words(&a.title), title_words(&b.title));
    if (a.start - b.start).abs() > Duration::minutes(TWIN_GAP) {
        let day = |e: &Event| e.start.with_timezone(&Berlin).date_naive();
        let (short, long) = if wa.len() <= wb.len() {
            (&wa, &wb)
        } else {
            (&wb, &wa)
        };
        return day(a) == day(b) && !short.is_empty() && short.is_subset(long);
    }
    if wa.is_empty() || wb.is_empty() {
        return a.title.to_lowercase() == b.title.to_lowercase();
    }
    let shared = wa.intersection(&wb).count();
    shared * 10 >= wa.len().min(wb.len()) * 6
}

/// Events arrive in source order, which decides whose title and canonical link
/// win; the sort is stable, so that order survives among equal start times.
pub fn merge(mut events: Vec<Event>) -> Vec<Event> {
    events.sort_by_key(|e| e.start);
    let mut merged: Vec<Event> = Vec::new();
    for e in events {
        // Sorted by start, so a twin can only be among the last day's.
        let twin = merged
            .iter_mut()
            .rev()
            .take_while(|m| e.start - m.start <= Duration::hours(24))
            .find(|m| same_event(m, &e));
        let Some(twin) = twin else {
            merged.push(e);
            continue;
        };
        for l in e.links {
            if !twin.links.iter().any(|(_, u)| *u == l.1) {
                twin.links.push(l);
            }
        }
        for g in e.groups {
            if !twin.groups.contains(&g) {
                twin.groups.push(g);
            }
        }
        if e.excerpt.len() > twin.excerpt.len() {
            twin.excerpt = e.excerpt;
        }
        if e.location.len() > twin.location.len() {
            twin.location = e.location;
        }
        twin.signups = twin.signups.max(e.signups);
        twin.end = twin.end.or(e.end);
        twin.online |= e.online;
        // Posted somewhere after all: then it is not chat-only.
        twin.chat &= e.chat;
    }
    for e in &mut merged {
        tag_by_title(e);
        e.groups
            .sort_by_key(|g| GROUPS.iter().position(|(k, _)| k == g));
    }
    merged
}

/// An EA or LW/ACX event whose title names the other community is theirs
/// too: EA Munich posts "ACX Spring Meetups Everywhere" to its own pages.
fn tag_by_title(e: &mut Event) {
    if !e.in_any(&["acx", "ea"]) {
        return;
    }
    let lower = e.title.to_lowercase();
    let words: HashSet<&str> = lower.split(|c: char| !c.is_alphanumeric()).collect();
    let acx = [
        "acx",
        "lesswrong",
        "lw",
        "rationalist",
        "rationalists",
        "rationality",
    ];
    if acx.iter().any(|w| words.contains(w)) && !e.groups.iter().any(|g| g == "acx") {
        e.groups.push("acx".into());
    }
    let ea = words.contains("ea") || lower.contains("effective altruism");
    if ea && !e.groups.iter().any(|g| g == "ea") {
        e.groups.push("ea".into());
    }
}

// Permalinks

fn slugify(title: &str) -> String {
    let mut out = String::new();
    for c in title.to_lowercase().chars() {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'ß' => out.push_str("ss"),
            c if c.is_ascii_alphanumeric() => out.push(c),
            _ if !out.ends_with('-') => out.push('-'),
            _ => {}
        }
    }
    let out = out.trim_matches('-');
    // Cut at a word boundary so ids stay readable.
    let mut cut = String::new();
    for word in out.split('-') {
        if !cut.is_empty() && cut.len() + word.len() >= 48 {
            break;
        }
        if !cut.is_empty() {
            cut.push('-');
        }
        cut.push_str(word);
    }
    if cut.is_empty() { "event".into() } else { cut }
}

/// `2026-09-26-petrov-day-ritual-munich`: the Munich date plus the title, so a
/// link survives the event moving between feeds. Same-day namesakes get `-2`.
pub fn assign_ids(events: &mut [Event]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    for e in events.iter_mut() {
        let base = format!(
            "{}-{}",
            e.start.with_timezone(&Berlin).format("%Y-%m-%d"),
            slugify(&e.title)
        );
        let n = seen.entry(base.clone()).or_insert(0);
        *n += 1;
        e.id = if *n == 1 { base } else { format!("{base}-{n}") };
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use chrono::TimeZone;

    pub fn ev(title: &str, minutes: i64, label: &str) -> Event {
        let t = Utc.with_ymd_and_hms(2026, 9, 26, 16, 0, 0).unwrap() + Duration::minutes(minutes);
        let url = format!("https://example.org/{label}/{}", slugify(title));
        Event::new(Raw {
            title,
            start: t,
            end: None,
            location: "",
            online: false,
            text: "",
            group: "ea",
            label,
            url: &url,
        })
    }

    #[test]
    fn cross_posts_match_despite_different_titles() {
        let same = [
            (
                "Petrov Day",
                "Petrov Day Ritual: Munich (multiplayer Petrov)",
            ),
            (
                "ACX Meetups Everywhere Munich",
                "Munich ACX Meetup (Everywhere, Fall 2026)",
            ),
            ("EA Social: October", "EA Munich Social - October"),
        ];
        for (a, b) in same {
            assert!(same_event(&ev(a, 0, "A"), &ev(b, 10, "B")), "{a} | {b}");
        }
    }

    #[test]
    fn different_events_stay_apart() {
        let different = [("Board games", "EA Social"), ("EA Intro Talk", "EA Dinner")];
        for (a, b) in different {
            assert!(!same_event(&ev(a, 0, "A"), &ev(b, 0, "B")), "{a} | {b}");
        }
        // The same title on another day is another event.
        assert!(!same_event(
            &ev("Petrov Day", 0, "A"),
            &ev("Petrov Day", 24 * 60, "B")
        ));
    }

    #[test]
    fn merge_keeps_every_link_and_group_and_the_best_details() {
        let mut a = ev("Petrov Day Ritual: Munich", 0, "LessWrong");
        a.groups = vec!["acx".into()];
        a.location = "Munich, Germany".into();
        let mut b = ev(
            "Petrov Day Ritual: Munich (multiplayer Petrov)",
            15,
            "EA Forum",
        );
        b.location = "Café X, Leopoldstr. 1, München".into();
        let merged = merge(vec![a, b, ev("Board games", 0, "LessWrong")]);
        assert_eq!(merged.len(), 2);
        let petrov = merged.iter().find(|e| e.title.contains("Petrov")).unwrap();
        assert_eq!(petrov.links.len(), 2);
        assert_eq!(petrov.groups, ["acx", "ea"]);
        assert_eq!(petrov.location, "Café X, Leopoldstr. 1, München");
    }

    #[test]
    fn a_moved_time_still_matches_on_the_same_day() {
        let posted = ev("Community picnic", 0, "EA Forum");
        let moved = ev("EA Community Picnic", 90, "Meetup");
        assert!(same_event(&posted, &moved));
        // A different event later the same day stays apart.
        assert!(!same_event(&posted, &ev("Board games", 90, "Meetup")));
        // So does the same title on another day.
        assert!(!same_event(
            &posted,
            &ev("Community picnic", 24 * 60, "Meetup")
        ));
        assert_eq!(merge(vec![posted, moved]).len(), 1);
    }

    #[test]
    fn titles_naming_the_other_community_join_it() {
        let mut spring = ev("ACX Spring Meetups Everywhere 2026", 0, "EA Forum");
        spring.groups = vec!["ea".into()];
        let mut reading = ev("Rationality and the EA mindset", 60, "Philosophia");
        reading.groups = vec!["philosophia".into()];
        let merged = merge(vec![spring, reading]);
        assert_eq!(merged[0].groups, ["acx", "ea"]);
        // Only EA and LW/ACX events are retagged.
        assert_eq!(merged[1].groups, ["philosophia"]);
    }

    #[test]
    fn ids_are_readable_stable_and_unique() {
        let mut events = vec![
            ev("Petrov Day Ritual: Munich (multiplayer Petrov)", 0, "A"),
            ev("Board games", 0, "B"),
            ev("Board games", 120, "C"),
            ev("Führung durch die Glyptothek", 0, "D"),
        ];
        assign_ids(&mut events);
        assert_eq!(
            events[0].id,
            "2026-09-26-petrov-day-ritual-munich-multiplayer-petrov"
        );
        assert_eq!(events[1].id, "2026-09-26-board-games");
        assert_eq!(events[2].id, "2026-09-26-board-games-2");
        assert_eq!(events[3].id, "2026-09-26-fuehrung-durch-die-glyptothek");
    }

    #[test]
    fn feed_urls_and_control_characters_do_not_survive() {
        let mut bad = ev("Evil", 0, "A");
        bad.links = vec![("X".into(), "javascript:alert(1)".into())];
        let mut ok = ev("Reading\rATTACH:x group", 60, "B");
        ok.location = "Room 1\nX-FOO:bar, Germany".into();
        ok.links
            .push(("Y".into(), "https://example.org/a\nATTACH:x".into()));
        let got = sanitize(vec![bad, ok]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].links.len(), 1);
        assert_eq!(got[0].title, "ReadingATTACH:x group");
        assert_eq!(got[0].location, "Room 1X-FOO:bar");
    }

    #[test]
    fn old_caches_with_group_names_still_load() {
        let mut e = ev("Dinner", 0, "A");
        e.groups = vec!["EA Munich".into(), "Somebody else".into()];
        assert_eq!(sanitize(vec![e])[0].groups, ["ea"]);
    }

    #[test]
    fn excerpt_cuts_at_a_word() {
        let e = excerpt(&"word ".repeat(100));
        assert!(e.ends_with(" …") && e.chars().count() <= EXCERPT_CHARS + 2);
    }
}
