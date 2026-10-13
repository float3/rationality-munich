//! The statistics page: how often the groups meet, how many come, and how
//! that has changed.
//!
//! The older version of this page drew its charts in the browser from a
//! copy of every event embedded in the HTML. These pages run no script, so
//! the charts are SVG written here, and the tables below them hold the
//! same numbers for anyone reading without images.
//!
//! Counted over every group the site collects, which is what the past
//! events page lists, so the two pages agree on how many there were.

use std::collections::BTreeMap;

use chrono::{Datelike, Weekday};
use chrono_tz::Europe::Berlin;

use super::plural;
use crate::event::{Event, GROUPS};
use crate::kind::kind;
use crate::render::html_escape;
use crate::turnout;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DAYS: [(Weekday, &str); 7] = [
    (Weekday::Mon, "Monday"),
    (Weekday::Tue, "Tuesday"),
    (Weekday::Wed, "Wednesday"),
    (Weekday::Thu, "Thursday"),
    (Weekday::Fri, "Friday"),
    (Weekday::Sat, "Saturday"),
    (Weekday::Sun, "Sunday"),
];

/// How many months of the chart to draw. Longer than the groups have been
/// running, so it starts where they did.
const CHART_MONTHS: usize = 12 * 12;

fn year_of(e: &Event) -> i32 {
    e.start.with_timezone(&Berlin).year()
}

/// A bar per month, with a twelve-month average over the top. The y axis
/// starts at nought and is labelled with the tallest bar, so the shape can
/// be read without hovering anything.
fn over_time(months: &[(i32, u32, usize)]) -> String {
    if months.is_empty() {
        return String::new();
    }
    let (w, h) = (960.0_f64, 260.0_f64);
    let (left, bottom, top) = (34.0_f64, 28.0_f64, 14.0_f64);
    let max = months.iter().map(|m| m.2).max().unwrap_or(1).max(1) as f64;
    let step = (w - left) / months.len() as f64;
    let plot = h - bottom - top;

    let bars: String = months
        .iter()
        .enumerate()
        .map(|(i, (year, month, n))| {
            let height = plot * *n as f64 / max;
            format!(
                r#"<rect x="{x:.1}" y="{y:.1}" width="{bw:.1}" height="{height:.1}"><title>{label} {year}: {n} event{s}</title></rect>"#,
                x = left + i as f64 * step + step * 0.15,
                y = top + plot - height,
                bw = (step * 0.7).max(1.0),
                label = MONTHS[*month as usize],
                s = plural(*n),
            )
        })
        .collect();

    // Twelve months either side of each point, so a quiet August does not
    // look like the groups stopped.
    let average: Vec<f64> = (0..months.len())
        .map(|i| {
            let from = i.saturating_sub(5);
            let to = (i + 6).min(months.len());
            let window = &months[from..to];
            window.iter().map(|m| m.2).sum::<usize>() as f64 / window.len() as f64
        })
        .collect();
    let line: String = average
        .iter()
        .enumerate()
        .map(|(i, v)| {
            format!(
                "{x:.1},{y:.1} ",
                x = left + i as f64 * step + step / 2.0,
                y = top + plot - plot * v / max,
            )
        })
        .collect();

    let ticks: String = months
        .iter()
        .enumerate()
        .filter(|(_, (_, month, _))| *month == 0)
        .map(|(i, (year, _, _))| {
            format!(
                r#"<text class="tick" x="{x:.1}" y="{y:.1}">{year}</text>"#,
                x = left + i as f64 * step + step / 2.0,
                y = h - 8.0,
            )
        })
        .collect();

    format!(
        r##"<figure class="chart"><svg viewBox="0 0 {w} {h}" role="img" aria-label="Events per month, with a rolling average">
<line class="axis" x1="{left}" y1="{base:.1}" x2="{w}" y2="{base:.1}"/>
<text class="tick" x="0" y="{top_label:.1}">{max:.0}</text>
<text class="tick" x="0" y="{base:.1}">0</text>
<g class="bars">{bars}</g>
<polyline class="trend" points="{line}"/>
{ticks}
</svg><figcaption>Events per month, and the average over the year around each month.</figcaption></figure>"##,
        base = top + plot,
        top_label = top + 4.0,
    )
}

/// Reported headcounts over time: a dot per event and the average of the
/// five around it.
fn attendance_chart(points: &[(String, u32)]) -> String {
    if points.len() < 5 {
        return String::new();
    }
    let (w, h) = (960.0_f64, 220.0_f64);
    let (left, bottom, top) = (34.0_f64, 24.0_f64, 14.0_f64);
    let max = points.iter().map(|p| p.1).max().unwrap_or(1).max(1) as f64;
    let step = (w - left) / points.len() as f64;
    let plot = h - bottom - top;
    let at = |i: usize, v: f64| {
        (
            left + i as f64 * step + step / 2.0,
            top + plot - plot * v / max,
        )
    };

    let dots: String = points
        .iter()
        .enumerate()
        .map(|(i, (when, n))| {
            let (x, y) = at(i, *n as f64);
            format!(
                r#"<circle cx="{x:.1}" cy="{y:.1}" r="2.6"><title>{when}: {n}</title></circle>"#,
                when = html_escape(when),
            )
        })
        .collect();
    let line: String = (0..points.len())
        .map(|i| {
            let from = i.saturating_sub(2);
            let to = (i + 3).min(points.len());
            let window = &points[from..to];
            let mean = window.iter().map(|p| p.1).sum::<u32>() as f64 / window.len() as f64;
            let (x, y) = at(i, mean);
            format!("{x:.1},{y:.1} ")
        })
        .collect();

    format!(
        r##"<figure class="chart"><svg viewBox="0 0 {w} {h}" role="img" aria-label="How many came to each event, oldest first">
<line class="axis" x1="{left}" y1="{base:.1}" x2="{w}" y2="{base:.1}"/>
<text class="tick" x="0" y="{top_label:.1}">{max:.0}</text>
<text class="tick" x="0" y="{base:.1}">0</text>
<g class="dots">{dots}</g>
<polyline class="trend" points="{line}"/>
</svg><figcaption>How many came to each event with a count, oldest first, and the average of the five around it. Report one on the <a href="/calendar/past/">past events</a> page.</figcaption></figure>"##,
        base = top + plot,
        top_label = top + 4.0,
    )
}

/// A cell shaded by how its count compares with the busiest one.
fn heat(n: usize, max: usize) -> String {
    if n == 0 {
        return r#"<td class="zero">·</td>"#.to_string();
    }
    let pct = 12 + n * 58 / max.max(1);
    format!(
        r#"<td class="heat" style="background: color-mix(in srgb, var(--mark) {pct}%, transparent)">{n}</td>"#
    )
}

fn bar(n: usize, max: usize) -> String {
    format!(
        r#"<span class="meter"><span style="width: {}%"></span></span>"#,
        n * 100 / max.max(1)
    )
}

fn median(values: &mut [u32]) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

/// `events` are the ones that have already happened, oldest first.
pub fn page(events: &[&Event]) -> String {
    let shown: Vec<&&Event> = events.iter().collect();
    if shown.is_empty() {
        return r#"<main id="main" class="wrap calendar-page"><div class="page-heading"><p class="eyebrow">STATISTICS</p><h1>Nothing counted yet</h1></div></main>"#.to_string();
    }
    let local = |e: &Event| e.start.with_timezone(&Berlin);
    let first_year = shown.iter().map(|e| year_of(e)).min().unwrap_or(0);
    let last = shown.last().expect("not empty");

    // Every month from the first event to the last, so a gap shows as a gap.
    let mut counts: BTreeMap<(i32, u32), usize> = BTreeMap::new();
    for e in &shown {
        let t = local(e);
        *counts.entry((t.year(), t.month0())).or_default() += 1;
    }
    let (first, last_month) = (
        *counts.keys().next().expect("not empty"),
        *counts.keys().next_back().expect("not empty"),
    );
    let mut months: Vec<(i32, u32, usize)> = Vec::new();
    let (mut y, mut m) = first;
    while (y, m) <= last_month && months.len() < CHART_MONTHS {
        months.push((y, m, counts.get(&(y, m)).copied().unwrap_or(0)));
        if m == 11 {
            y += 1;
            m = 0;
        } else {
            m += 1;
        }
    }

    let busiest = months.iter().max_by_key(|m| m.2);
    let summary = format!(
        "<b>{n}</b> event{s} since {first_year}{busiest}.",
        n = shown.len(),
        s = plural(shown.len()),
        busiest = match busiest.filter(|b| b.2 > 0) {
            Some((year, month, n)) =>
                format!(", the most in {} {year}, with {n}", MONTHS[*month as usize]),
            None => String::new(),
        },
    );

    // Per year and group. An event several groups held counts for each of
    // them, and once in the total.
    let years: Vec<i32> = {
        let mut all: Vec<i32> = shown.iter().map(|e| year_of(e)).collect();
        all.sort_unstable();
        all.dedup();
        all.reverse();
        all
    };
    let max_year = years
        .iter()
        .map(|y| shown.iter().filter(|e| year_of(e) == *y).count())
        .max()
        .unwrap_or(0);
    let head: String = GROUPS
        .iter()
        .map(|(_, name)| format!(r#"<th scope="col">{}</th>"#, html_escape(name)))
        .collect();
    let year_rows: String = years
        .iter()
        .map(|year| {
            let cells: String = GROUPS
                .iter()
                .map(|(key, _)| {
                    let n = shown
                        .iter()
                        .filter(|e| year_of(e) == *year && e.in_any(&[key]))
                        .count();
                    format!("<td>{n}</td>")
                })
                .collect();
            let total = shown.iter().filter(|e| year_of(e) == *year).count();
            format!(
                r#"<tr><th scope="row">{year}</th>{cells}<td class="total">{total}</td><td class="wide">{}</td></tr>"#,
                bar(total, max_year)
            )
        })
        .collect();

    // Year by month.
    let grid: BTreeMap<i32, [usize; 12]> = {
        let mut grid: BTreeMap<i32, [usize; 12]> = years.iter().map(|y| (*y, [0; 12])).collect();
        for e in &shown {
            let t = local(e);
            grid.get_mut(&t.year()).expect("every year has a row")[t.month0() as usize] += 1;
        }
        grid
    };
    let max_month = grid.values().flatten().copied().max().unwrap_or(0);
    let month_head: String = MONTHS
        .iter()
        .map(|m| format!(r#"<th scope="col">{m}</th>"#))
        .collect();
    let month_rows: String = grid
        .iter()
        .rev()
        .map(|(year, counts)| {
            let cells: String = counts.iter().map(|n| heat(*n, max_month)).collect();
            format!(r#"<tr><th scope="row">{year}</th>{cells}</tr>"#)
        })
        .collect();

    // Weekday.
    let per_day: Vec<usize> = DAYS
        .iter()
        .map(|(d, _)| shown.iter().filter(|e| local(e).weekday() == *d).count())
        .collect();
    let max_day = per_day.iter().copied().max().unwrap_or(0);
    let day_rows: String = DAYS
        .iter()
        .zip(&per_day)
        .map(|((_, name), n)| {
            format!(
                r#"<tr><th scope="row">{name}</th><td class="total">{n}</td><td class="wide">{}</td></tr>"#,
                bar(*n, max_day)
            )
        })
        .collect();

    // By kind of event.
    let mut kinds: BTreeMap<&'static str, (usize, Vec<u32>)> = BTreeMap::new();
    for e in &shown {
        let entry = kinds.entry(kind(&e.title)).or_default();
        entry.0 += 1;
        if let Some(n) = e.headcount() {
            entry.1.push(n);
        }
    }
    let max_kind = kinds.values().map(|(n, _)| *n).max().unwrap_or(0);
    let mut by_kind: Vec<(&str, usize, Option<u32>)> = kinds
        .into_iter()
        .map(|(name, (n, mut counts))| (name, n, median(&mut counts)))
        .collect();
    by_kind.sort_by_key(|(_, n, _)| std::cmp::Reverse(*n));
    let kind_rows: String = by_kind
        .iter()
        .map(|(name, n, typical)| {
            format!(
                r#"<tr><th scope="row">{name}</th><td class="total">{n}</td><td>{typical}</td><td class="wide">{bar}</td></tr>"#,
                name = html_escape(name),
                typical = typical.map_or("—".to_string(), |t| t.to_string()),
                bar = bar(*n, max_kind),
            )
        })
        .collect();

    // Attendance over time.
    let mut points: Vec<(String, u32)> = shown
        .iter()
        .filter_map(|e| {
            e.headcount()
                .map(|n| (local(e).format("%-d %b %Y").to_string(), n))
        })
        .collect();
    points.truncate(400);
    let per_signup = match turnout::came_per_signup(events.iter().copied()) {
        (ratio, n) if n >= 3 => format!(
            r#"<p class="section-note">About {ratio:.1} came per sign-up at the {n} events with both. Each group's ratio, per kind of event, is what the expected turnout of its upcoming events is calculated from.</p>"#
        ),
        _ => String::new(),
    };

    format!(
        r##"<main id="main" class="wrap calendar-page">
<div class="page-heading"><p class="eyebrow">STATISTICS</p><div class="title-actions"><h1>How often the groups meet</h1></div><p>{summary}</p></div>
<div class="calendar-context"><span><span class="status-dot"></span> Counted to {last_day}</span><span>Every group this site collects</span></div>
{over_time}
<p class="section-note">Some events were never posted anywhere, so the real numbers are higher. Events arranged only in the group chats are counted from spring 2023; casual meetups like lunch and coworking are not.</p>
<h2 class="year-heading">How many came</h2>
{attendance}
{per_signup}
<h2 class="year-heading">Per year</h2>
<div class="scroll"><table class="figures"><thead><tr><td></td>{head}<th scope="col">Total</th><td class="wide"></td></tr></thead><tbody>{year_rows}</tbody></table></div>
<p class="section-note">An event two groups held counts for each of them, and once in the total.</p>
<h2 class="year-heading">Per month</h2>
<div class="scroll"><table class="figures months"><thead><tr><td></td>{month_head}</tr></thead><tbody>{month_rows}</tbody></table></div>
<h2 class="year-heading">Per weekday</h2>
<table class="figures"><tbody>{day_rows}</tbody></table>
<h2 class="year-heading">By kind</h2>
<div class="scroll"><table class="figures"><thead><tr><td></td><th scope="col">Events</th><th scope="col">Typical</th><td class="wide"></td></tr></thead><tbody>{kind_rows}</tbody></table></div>
<p class="section-note">Kinds are read from the titles. Typical is the middle headcount, as reported, else sign-ups.</p>
<aside class="coverage-note"><h3>Where the numbers come from</h3><p>Everything on this page is counted from the events the site collects, including the ones kept by hand in the repository. Headcounts are what organisers gave or what people reported on the past events page.</p><div class="actions"><a class="text-link" href="/calendar/">Upcoming events</a><a class="text-link" href="/calendar/past/">Past events</a></div></aside>
</main>"##,
        over_time = over_time(&months),
        attendance = attendance_chart(&points),
        last_day = local(last).format("%-d %B %Y"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::tests::ev;

    #[test]
    fn it_counts_events_without_double_counting_the_total() {
        let mut shared = ev("Petrov Day", 0, "A");
        shared.groups = vec!["acx".into(), "ea".into()];
        let solo = ev("Dinner", 120, "B");
        let html = page(&[&shared, &solo]);
        assert!(html.contains("<b>2</b> events since 2026"));
        assert!(html.contains("the most in Sep 2026, with 2"));
    }

    /// The past events page lists every group, so these count them too:
    /// two pages that disagree about how many events there were is worse
    /// than a wide table.
    #[test]
    fn every_group_is_counted() {
        let mut reading = ev("Leviathan by Hobbes", 0, "Philosophia");
        reading.groups = vec!["philosophia".into()];
        let html = page(&[&reading, &ev("Dinner", 0, "B")]);
        assert!(html.contains("<b>2</b> events since 2026"));
    }

    #[test]
    fn the_page_runs_no_script() {
        let html = page(&[&ev("Dinner", 0, "B")]);
        assert!(!html.contains("<script"));
        assert!(!html.contains("hidden"));
    }
}
