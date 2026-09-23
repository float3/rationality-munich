//! Builds rationality-munich.com/calendar from the Munich groups' event feeds.
//!
//! Fetches past and upcoming events from LessWrong, the EA Forum, Meetup, Luma
//! and Philosophia's Google Calendar, merges events that were cross-posted,
//! and writes static pages and iCalendar files. Visitors never talk to those
//! sites; only this job does.
//!
//! Output, under `$STATE_DIRECTORY/site/`, served at /calendar/:
//!
//! - `index.html`, `past/index.html`, `stats/index.html`: the pages
//! - `feed.xml`: an Atom feed of upcoming events, for feed readers
//! - `feeds/<groups>.ics`: one feed per combination of groups
//!   (`acx+ea.ics`), plus `all.ics`
//! - `e/<id>.ics`: one file per event, for "Add to calendar"
//!
//! Each source's last good result is cached, so one site being down leaves its
//! events in place, and every past event is kept in `archive.json`, so events
//! that drop out of a feed once they happen stay in the history.

mod event;
mod feed;
mod ics;
mod render;
mod sources;
mod stats;

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};

use event::{COMBO_GROUPS, Event, GROUPS, assign_ids, group_name, merge, sanitize};
use render::{BASE, Kind, Page};
use sources::Res;

/// How long a past event stays in the subscription feeds, so it does not
/// vanish from people's calendars the moment it ends.
const FEED_KEEPS_PAST_DAYS: i64 = 60;

fn collect(
    agent: &ureq::Agent,
    state: &Path,
    now: DateTime<Utc>,
) -> Res<(Vec<Event>, Vec<String>)> {
    let cache = state.join("sources");
    fs::create_dir_all(&cache)?;
    let (mut events, mut stale) = (Vec::new(), Vec::<String>::new());
    for source in sources::all() {
        let path = cache.join(format!("{}.json", source.key));
        match (source.fetch)(agent, now) {
            Ok(got) => {
                fs::write(&path, serde_json::to_string(&got)?)?;
                events.extend(got);
            }
            Err(err) => {
                eprintln!("{}: {err}", source.key);
                if !stale.iter().any(|s| s == source.label) {
                    stale.push(source.label.to_string());
                }
                if let Ok(cached) = fs::read_to_string(&path) {
                    events.extend(serde_json::from_str::<Vec<Event>>(&cached).unwrap_or_default());
                }
            }
        }
    }
    stale.sort();
    Ok((events, stale))
}

fn write(path: &Path, text: &str) -> Res<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, text)?;
    Ok(())
}

/// The feeds to write, as sets of groups: every non-empty combination of
/// `COMBO_GROUPS` (`acx`, `acx+ea`, …), then each other group on its own.
fn group_sets() -> Vec<Vec<&'static str>> {
    let combos = (1..1u32 << COMBO_GROUPS.len()).map(|mask| {
        COMBO_GROUPS
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, k)| *k)
            .collect()
    });
    let singles = GROUPS
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| !COMBO_GROUPS.contains(k))
        .map(|k| vec![k]);
    combos.chain(singles).collect()
}

fn build_site(dir: &Path, events: &[Event], stale: &[String], now: DateTime<Utc>) -> Res<()> {
    let (upcoming, mut past): (Vec<&Event>, Vec<&Event>) =
        events.iter().partition(|e| e.end_or_default() >= now);
    past.reverse();
    let unannounced = sources::unannounced(now);

    write(
        &dir.join("index.html"),
        &render::page(&Page {
            kind: Kind::Upcoming,
            events: upcoming.clone(),
            stale,
            now,
            unannounced: &unannounced,
        }),
    )?;
    write(
        &dir.join("past/index.html"),
        &render::page(&Page {
            kind: Kind::Past,
            events: past.clone(),
            stale,
            now,
            unannounced: &unannounced,
        }),
    )?;
    write(
        &dir.join("stats/index.html"),
        &render::page(&Page {
            kind: Kind::Stats,
            events: past,
            stale,
            now,
            unannounced: &unannounced,
        }),
    )?;

    write(&dir.join("feed.xml"), &feed::atom(&upcoming, now))?;

    let recent = now - Duration::days(FEED_KEEPS_PAST_DAYS);
    let feed_events: Vec<&Event> = events
        .iter()
        .filter(|e| e.end_or_default() >= recent)
        .collect();
    for set in group_sets() {
        let chosen: Vec<&Event> = feed_events
            .iter()
            .copied()
            .filter(|e| e.in_any(&set))
            .collect();
        let names: Vec<&str> = set.iter().map(|k| group_name(k)).collect();
        let name = format!("Rationality Munich: {}", names.join(", "));
        write(
            &dir.join(format!("feeds/{}.ics", set.join("+"))),
            &ics::calendar(&name, &chosen, now, BASE),
        )?;
    }
    // Every group; also served as /calendar.ics.
    write(
        &dir.join("feeds/all.ics"),
        &ics::calendar("Rationality Munich", &feed_events, now, BASE),
    )?;

    for e in events {
        write(
            &dir.join(format!("e/{}.ics", e.id)),
            &ics::calendar(&e.title, &[e], now, BASE),
        )?;
    }
    Ok(())
}

/// Builds next to the live copy and swaps it in, so nginx never serves a
/// half-written site and files for vanished events disappear.
fn publish(state: &Path, events: &[Event], stale: &[String], now: DateTime<Utc>) -> Res<()> {
    let (site, new, old) = (
        state.join("site"),
        state.join("site.new"),
        state.join("site.old"),
    );
    let _ = fs::remove_dir_all(&new);
    build_site(&new, events, stale, now)?;
    let _ = fs::remove_dir_all(&old);
    if site.exists() {
        fs::rename(&site, &old)?;
    }
    fs::rename(&new, &site)?;
    let _ = fs::remove_dir_all(&old);
    // Left over from before the site moved into its own directory.
    for stray in ["index.html", "calendar.ics"] {
        let _ = fs::remove_file(state.join(stray));
    }
    Ok(())
}

fn main() -> Res<()> {
    let state = PathBuf::from(std::env::var("STATE_DIRECTORY").unwrap_or_else(|_| "out".into()));
    fs::create_dir_all(&state)?;
    let now = Utc::now();
    let (fresh, stale) = collect(&sources::agent(), &state, now)?;

    let archive_path = state.join("archive.json");
    let archive: Vec<Event> = fs::read_to_string(&archive_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Fresh data first: among cross-posts, the first one seen gives the title
    // and the canonical link.
    let mut events = merge(sanitize(
        fresh
            .into_iter()
            .chain(archive)
            .chain(sources::history())
            .collect(),
    ));
    assign_ids(&mut events);

    let past: Vec<&Event> = events.iter().filter(|e| e.end_or_default() < now).collect();
    fs::write(&archive_path, serde_json::to_string(&past)?)?;

    publish(&state, &events, &stale, now)?;
    let upcoming = events.len() - past.len();
    println!(
        "{upcoming} upcoming and {} past events{}",
        past.len(),
        if stale.is_empty() {
            String::new()
        } else {
            format!(", stale: {}", stale.join(", "))
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_feed_for_every_combination_of_groups() {
        let names: Vec<String> = group_sets().iter().map(|s| s.join("+")).collect();
        let singles = GROUPS.len() - COMBO_GROUPS.len();
        assert_eq!(names.len(), (1 << COMBO_GROUPS.len()) - 1 + singles);
        assert!(names.contains(&"acx+ea+philosophia".to_string()));
        assert!(names.contains(&"agi".to_string()) && !names.contains(&"acx+agi".to_string()));
        assert!(names.contains(&"acx+ea".to_string()));
        assert!(names.contains(&"philosophia".to_string()));
    }
}
