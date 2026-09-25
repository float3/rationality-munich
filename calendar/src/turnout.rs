//! Expected turnout for upcoming events: from their sign-ups, and from how
//! many came to earlier events of the same series. Visitors' reports of how
//! many actually came calibrate both, so the estimate improves as reports
//! come in.

use chrono::{DateTime, Utc};

use crate::event::{Event, title_words};

/// The came-per-sign-up ratio starts at 1, as if this many sign-ups had all
/// come, so the first few reports move it only a little.
const PRIOR_SIGNUPS: f64 = 10.0;
/// How many of a series' latest events are averaged for the next one.
const SERIES_WINDOW: usize = 5;
/// A series needs at least this many past events with a headcount.
const SERIES_MIN: usize = 2;

/// People who came per sign-up, and how many events that rests on: past
/// events with both a visitor report and sign-ups.
pub fn came_per_signup<'a>(events: impl IntoIterator<Item = &'a Event>) -> (f64, usize) {
    let (mut came, mut signed, mut n) = (0.0, 0.0, 0);
    for e in events {
        if let (Some(c), s) = (e.attended, e.signed_up())
            && s > 0
        {
            came += c.n as f64;
            signed += s as f64;
            n += 1;
        }
    }
    ((came + PRIOR_SIGNUPS) / (signed + PRIOR_SIGNUPS), n)
}

/// Same groups, and the shorter title's words mostly in the longer: "ACX
/// community dinner" and "Community Dinner", but not an EA dinner.
fn same_series(a: &Event, b: &Event) -> bool {
    if a.groups != b.groups {
        return false;
    }
    let (wa, wb) = (title_words(&a.title), title_words(&b.title));
    let shorter = wa.len().min(wb.len());
    shorter > 0 && wa.intersection(&wb).count() * 10 >= shorter * 6
}

/// How many came to a past event: the visitors' report, else its sign-ups
/// scaled by the ratio.
fn headcount(e: &Event, ratio: f64) -> Option<f64> {
    e.attended
        .map(|c| c.n as f64)
        .or_else(|| (e.signed_up() > 0).then_some(e.signed_up() as f64 * ratio))
}

/// Sets `expected` on every upcoming event there is a basis for: the larger
/// of its scaled sign-ups and its series' recent average, since sign-ups
/// often come in only on the day. `events` are sorted by start.
///
/// The ratio is the event's groups' own: people RSVP on Meetup far more
/// readily than in the ACX chat, so one group's habit says little about
/// another's.
pub fn estimate(events: &mut [Event], now: DateTime<Utc>) {
    let expected: Vec<Option<u32>> = events
        .iter()
        .map(|e| {
            if e.end_or_default() < now {
                return None;
            }
            let (ratio, _) = came_per_signup(
                events
                    .iter()
                    .filter(|p| p.groups.iter().any(|g| e.groups.contains(g))),
            );
            let recent: Vec<f64> = events
                .iter()
                .rev()
                .filter(|p| p.end_or_default() < now && same_series(p, e))
                .filter_map(|p| headcount(p, ratio))
                .take(SERIES_WINDOW)
                .collect();
            let series = (recent.len() >= SERIES_MIN)
                .then(|| recent.iter().sum::<f64>() / recent.len() as f64);
            let signups = (e.signed_up() > 0).then_some(e.signed_up() as f64 * ratio);
            let best = series
                .into_iter()
                .chain(signups)
                .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x))))?;
            Some((best.round() as u32).max(1))
        })
        .collect();
    for (e, x) in events.iter_mut().zip(expected) {
        e.expected = x;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Raw;

    fn event(title: &str, start: &str, group: &str, signups: u32, came: Option<u32>) -> Event {
        let mut e = Event::new(Raw {
            title,
            start: DateTime::parse_from_rfc3339(start)
                .unwrap()
                .with_timezone(&Utc),
            end: None,
            location: "",
            online: false,
            text: "",
            group,
            label: "Meetup",
            url: "https://www.meetup.com/x/events/1/",
        });
        e.signups = signups;
        e.attended = came.map(|n| crate::event::Came { n, at_least: false });
        e
    }

    #[test]
    fn the_ratio_starts_at_one_and_follows_reports() {
        assert_eq!(came_per_signup(&[]), (1.0, 0));
        // 30 came for 10 sign-ups: (30 + 10) / (10 + 10).
        let past = [event(
            "Dinner",
            "2026-09-01T18:30:00+02:00",
            "ea",
            10,
            Some(30),
        )];
        assert_eq!(came_per_signup(&past), (2.0, 1));
    }

    #[test]
    fn a_series_predicts_its_next_event_even_without_signups() {
        let now = DateTime::parse_from_rfc3339("2026-09-20T12:00:00+02:00")
            .unwrap()
            .with_timezone(&Utc);
        let mut events = vec![
            event(
                "ACX community dinner",
                "2026-08-26T18:30:00+02:00",
                "acx",
                0,
                Some(6),
            ),
            event(
                "Community Dinner",
                "2026-09-09T18:30:00+02:00",
                "acx",
                0,
                Some(10),
            ),
            // Another group's dinner is another series.
            event(
                "EA community dinner",
                "2026-09-10T18:30:00+02:00",
                "ea",
                0,
                Some(40),
            ),
            event(
                "ACX community dinner",
                "2026-09-23T18:30:00+02:00",
                "acx",
                3,
                None,
            ),
            event(
                "Talk on something new",
                "2026-09-24T18:30:00+02:00",
                "acx",
                4,
                None,
            ),
            event(
                "Something else",
                "2026-09-25T18:30:00+02:00",
                "acx",
                0,
                None,
            ),
            // Nobody reported for this group, so its sign-ups stand.
            event(
                "Book club",
                "2026-09-26T18:30:00+02:00",
                "culture",
                12,
                None,
            ),
        ];
        estimate(&mut events, now);
        assert_eq!(
            events[3].expected,
            Some(8),
            "the series beats 3 early sign-ups"
        );
        assert_eq!(events[4].expected, Some(4), "no series: the sign-ups");
        assert_eq!(events[5].expected, None, "no basis at all");
        assert_eq!(events[6].expected, Some(12), "another group's ratio");
        assert_eq!(events[1].expected, None, "past events keep their headcount");
    }
}
