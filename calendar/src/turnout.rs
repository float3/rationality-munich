//! Expected turnout for upcoming events: from their sign-ups, from how many
//! came to earlier events of the same series, and else from the group's
//! recent events of the same kind (dinners, talks, …; see `kind`). Reported
//! headcounts calibrate all of it, so the estimate improves as they come in.

use chrono::{DateTime, Utc};

use crate::event::{Event, title_words};
use crate::kind::kind;

/// The came-per-sign-up ratio starts at 1, as if this many sign-ups had all
/// come, so the first few reports move it only a little.
const PRIOR_SIGNUPS: f64 = 10.0;
/// How many of a series' latest events are averaged for the next one.
const SERIES_WINDOW: usize = 5;
/// A series needs at least this many past events with a headcount.
const SERIES_MIN: usize = 2;
/// How many of a group's latest events of a kind are averaged, and how many
/// with a headcount it takes.
const KIND_WINDOW: usize = 8;
const KIND_MIN: usize = 3;

/// People who came per sign-up, and how many events that rests on: past
/// events with both a visitor report and sign-ups.
pub fn came_per_signup<'a>(events: impl IntoIterator<Item = &'a Event>) -> (f64, usize) {
    let (came, signed, n) = sums(events);
    ((came + PRIOR_SIGNUPS) / (signed + PRIOR_SIGNUPS), n)
}

/// Headcounts and sign-ups summed over the events that have both.
fn sums<'a>(events: impl IntoIterator<Item = &'a Event>) -> (f64, f64, usize) {
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
    (came, signed, n)
}

/// Came per sign-up for events like `e`: its groups' events of its kind,
/// pulled towards the groups' ratio over all kinds while the kind has few
/// reports. People RSVP to a Meetup talk far more readily than to a dinner
/// arranged in a chat, and one group's habit says little about another's.
fn ratio_for(events: &[Event], e: &Event) -> f64 {
    let shares_group = |p: &&Event| p.groups.iter().any(|g| e.groups.contains(g));
    let (came, signed, _) = sums(events.iter().filter(shares_group));
    let group = (came + PRIOR_SIGNUPS) / (signed + PRIOR_SIGNUPS);
    let k = kind(&e.title);
    let (came, signed, _) = sums(
        events
            .iter()
            .filter(shares_group)
            .filter(|p| kind(&p.title) == k),
    );
    (came + PRIOR_SIGNUPS * group) / (signed + PRIOR_SIGNUPS)
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

/// Sets `expected` on every upcoming event there is a basis for. The usual
/// turnout comes from its series' recent average, else from its groups'
/// recent events of the same kind; its sign-ups, scaled, win when they are
/// more, since sign-ups often come in only on the day. `events` are sorted
/// by start.
pub fn estimate(events: &mut [Event], now: DateTime<Utc>) {
    let expected: Vec<Option<u32>> = events
        .iter()
        .map(|e| {
            if e.end_or_default() < now {
                return None;
            }
            let ratio = ratio_for(events, e);
            let average = |same: &dyn Fn(&Event) -> bool, window, min| {
                let recent: Vec<f64> = events
                    .iter()
                    .rev()
                    .filter(|p| p.end_or_default() < now && same(p))
                    .filter_map(|p| headcount(p, ratio))
                    .take(window)
                    .collect();
                (recent.len() >= min).then(|| recent.iter().sum::<f64>() / recent.len() as f64)
            };
            let k = kind(&e.title);
            let usual = average(&|p| same_series(p, e), SERIES_WINDOW, SERIES_MIN).or_else(|| {
                (k != "other")
                    .then(|| {
                        average(
                            &|p| {
                                kind(&p.title) == k && p.groups.iter().any(|g| e.groups.contains(g))
                            },
                            KIND_WINDOW,
                            KIND_MIN,
                        )
                    })
                    .flatten()
            });
            let signups = (e.signed_up() > 0).then_some(e.signed_up() as f64 * ratio);
            let best = usual
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
        assert_eq!(
            events[4].expected,
            Some(4),
            "no series or kind: the sign-ups"
        );
        assert_eq!(events[5].expected, None, "no basis at all");
        assert_eq!(events[6].expected, Some(12), "another group's ratio");
        assert_eq!(events[1].expected, None, "past events keep their headcount");
    }

    #[test]
    fn a_new_event_takes_after_its_kind() {
        let now = DateTime::parse_from_rfc3339("2026-09-20T12:00:00+02:00")
            .unwrap()
            .with_timezone(&Utc);
        let mut events = vec![
            event(
                "Talk: AI governance",
                "2026-06-01T19:00:00+02:00",
                "ea",
                0,
                Some(20),
            ),
            event(
                "Speaker series: biosecurity",
                "2026-07-01T19:00:00+02:00",
                "ea",
                0,
                Some(24),
            ),
            event(
                "Lecture on welfare economics",
                "2026-08-01T19:00:00+02:00",
                "ea",
                0,
                Some(22),
            ),
            event(
                "EA community dinner",
                "2026-09-01T19:00:00+02:00",
                "ea",
                0,
                Some(6),
            ),
            // Another group's talks are not this group's.
            event(
                "Talk: something",
                "2026-09-02T19:00:00+02:00",
                "culture",
                0,
                Some(90),
            ),
            event(
                "Talk: nuclear risk",
                "2026-09-25T19:00:00+02:00",
                "ea",
                3,
                None,
            ),
        ];
        estimate(&mut events, now);
        assert_eq!(
            events[5].expected,
            Some(22),
            "the group's talks, not its dinner"
        );
    }
}
