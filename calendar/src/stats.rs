//! How many events the groups have held: per year, per month and per weekday.
//!
//! The tables are rendered here for the default groups, so the page works
//! without JavaScript. Each cell names what it counts (`data-y`, `data-m`,
//! `data-g`, `data-wd`), and the page embeds one small row per event, so
//! switching groups recounts the same cells in the browser. `recount` in
//! page.html mirrors `shade`, `bar` and `summary` below.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Weekday};
use chrono_tz::Europe::Berlin;

use crate::event::{DEFAULT_GROUPS, Event, GROUPS};
use crate::render::html_escape;

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const DAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

/// A cell shaded by how its count compares with the busiest one.
fn shade(attrs: &str, n: usize, max: usize) -> String {
    if n == 0 {
        return format!(r#"<td {attrs} class="zero">·</td>"#);
    }
    let pct = 12 + n * 58 / max.max(1);
    let dark = if pct > 40 { " dark" } else { "" };
    format!(
        r#"<td {attrs} class="heat{dark}" style="background: color-mix(in srgb, var(--text) {pct}%, transparent)">{n}</td>"#
    )
}

fn bar(n: usize, max: usize) -> String {
    format!(
        r#"<span class="bar"><span style="width: {}%"></span></span>"#,
        n * 100 / max.max(1)
    )
}

fn summary(total: usize, first: Option<i32>, busiest: Option<(usize, i32, usize)>) -> String {
    let mut s = format!(
        "<b>{total}</b> events since {}.",
        first.map_or("the start".into(), |y| y.to_string())
    );
    if let Some((n, y, m)) = busiest.filter(|(n, _, _)| *n > 0) {
        s += &format!(" The busiest month so far was {} {y}, with {n}.", MONTHS[m]);
    }
    s
}

/// A column for a group that is off stays in the page, hidden.
fn hidden_unless_on(key: &str) -> &'static str {
    if DEFAULT_GROUPS.contains(&key) {
        ""
    } else {
        " hidden"
    }
}

/// `events` are the ones that already happened.
pub fn render(events: &[&Event]) -> String {
    let local = |e: &Event| e.start.with_timezone(&Berlin);
    let everything: Vec<&Event> = events.to_vec();
    // What the page shows before the visitor picks.
    let shown: Vec<&Event> = everything
        .iter()
        .copied()
        .filter(|e| e.in_any(DEFAULT_GROUPS))
        .collect();
    // Rows exist for every year any group met, so switching groups on only
    // ever fills in cells.
    let years: BTreeSet<i32> = everything.iter().map(|e| local(e).year()).collect();
    let in_year = |y: i32| shown.iter().filter(move |e| local(e).year() == y);

    // Per year and group. An event several groups held counts for each of
    // them, but once in the total.
    let max_year = years.iter().map(|y| in_year(*y).count()).max().unwrap_or(0);
    let head: String = GROUPS
        .iter()
        .map(|(key, name)| {
            format!(
                r#"<th scope="col" data-g="{key}"{}>{}</th>"#,
                hidden_unless_on(key),
                html_escape(name)
            )
        })
        .collect();
    let group_cell =
        |key: &str, n: usize| format!(r#"<td data-g="{key}"{}>{n}</td>"#, hidden_unless_on(key));
    let rows: String = years
        .iter()
        .rev()
        .map(|y| {
            let cells: String = GROUPS
                .iter()
                .map(|(key, _)| {
                    let n = everything
                        .iter()
                        .filter(|e| local(e).year() == *y && e.in_any(&[key]))
                        .count();
                    group_cell(key, n)
                })
                .collect();
            let total = in_year(*y).count();
            format!(
                r#"<tr data-y="{y}"><th scope="row">{y}</th>{cells}<td class="total">{total}</td><td class="wide">{}</td></tr>"#,
                bar(total, max_year)
            )
        })
        .collect();
    let total_cells: String = GROUPS
        .iter()
        .map(|(key, _)| group_cell(key, everything.iter().filter(|e| e.in_any(&[key])).count()))
        .collect();
    let by_year = format!(
        r#"<div class="scroll"><table class="years"><thead><tr><td></td>{head}<th scope="col">Total</th><td class="wide"></td></tr></thead><tbody>{rows}</tbody><tfoot><tr data-y="all"><th scope="row">All</th>{total_cells}<td class="total">{}</td><td class="wide"></td></tr></tfoot></table></div>"#,
        shown.len()
    );

    // Year by month.
    let mut grid: BTreeMap<i32, [usize; 12]> = years.iter().map(|y| (*y, [0; 12])).collect();
    for e in &shown {
        let t = local(e);
        grid.get_mut(&t.year()).expect("every year has a row")[t.month0() as usize] += 1;
    }
    let max_month = grid.values().flatten().copied().max().unwrap_or(0);
    let month_head: String = MONTHS
        .iter()
        .map(|m| format!(r#"<th scope="col">{m}</th>"#))
        .collect();
    let month_rows: String = grid
        .iter()
        .rev()
        .map(|(year, months)| {
            let cells: String = months
                .iter()
                .enumerate()
                .map(|(m, n)| shade(&format!(r#"data-y="{year}" data-m="{m}""#), *n, max_month))
                .collect();
            format!(r#"<tr><th scope="row">{year}</th>{cells}</tr>"#)
        })
        .collect();
    let by_month = format!(
        r#"<div class="scroll"><table class="months"><thead><tr><td></td>{month_head}</tr></thead><tbody>{month_rows}</tbody></table></div>"#
    );

    // Weekday.
    let per_day: Vec<usize> = DAYS
        .iter()
        .map(|d| shown.iter().filter(|e| local(e).weekday() == *d).count())
        .collect();
    let max_day = per_day.iter().copied().max().unwrap_or(0);
    let day_rows: String = DAYS
        .iter()
        .zip(&per_day)
        .enumerate()
        .map(|(i, (d, n))| {
            format!(
                r#"<tr data-wd="{i}"><th scope="row">{d}</th><td class="total">{n}</td><td class="wide">{}</td></tr>"#,
                bar(*n, max_day)
            )
        })
        .collect();
    let by_day = format!(r#"<table class="days"><tbody>{day_rows}</tbody></table>"#);

    let busiest = grid
        .iter()
        .flat_map(|(y, m)| m.iter().enumerate().map(move |(i, n)| (*n, *y, i)))
        .max();
    let first = shown.iter().map(|e| local(e).year()).min();

    // [year, month (0-11), weekday (0 = Monday), group keys, day of month,
    // attendance (reported, else sign-ups) or null] per event.
    type Row<'a> = (i32, u32, u32, &'a Vec<String>, u32, Option<u32>);
    let data: Vec<Row> = everything
        .iter()
        .map(|e| {
            let t = local(e);
            (
                t.year(),
                t.month0(),
                t.weekday().num_days_from_monday(),
                &e.groups,
                t.day(),
                e.headcount(),
            )
        })
        .collect();
    let data = serde_json::to_string(&data)
        .expect("serialisable")
        .replace("</", "<\\/");

    format!(
        r#"<p class="summary" id="summary">{summary}</p>
<p class="caveat">Some events were never posted anywhere, so the real numbers are higher. Events arranged only in the group chats are included since November 2025; casual meetups like lunch and coworking are not.</p>
<h2>Over time</h2>
<div class="compare" id="compare" hidden><label>Compare from <input type="month" id="since" value="2026-04"></label></div>
<div class="tiles" id="tiles"></div>
<figure class="chart" id="chart"><figcaption class="caveat">Events per month. Hover a month for its count.</figcaption></figure>
<h2>Attendance</h2>
<div class="tiles" id="att-tiles"></div>
<figure class="chart" id="att-chart"><figcaption class="caveat">Attendance per event as reported, else sign-ups, with the average of the last five. Report one on the <a href="/calendar/past/">past events</a> page.</figcaption></figure>
<noscript><p class="caveat">The charts need JavaScript; the tables below have the same event counts.</p></noscript>
<h2>Per year</h2>
{by_year}
<p class="caveat">Events held by several groups count for each, and once in the total.</p>
<h2>Per month</h2>
{by_month}
<h2>Per weekday</h2>
{by_day}
<script type="application/json" id="stats-data">{data}</script>"#,
        summary = summary(shown.len(), first, busiest),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::tests::ev;

    #[test]
    fn counts_per_year_and_group_without_double_counting_the_total() {
        let mut shared = ev("Petrov Day", 0, "A");
        shared.groups = vec!["acx".into(), "ea".into()];
        let solo = ev("Dinner", 120, "B");
        let html = render(&[&shared, &solo]);
        assert!(html.contains("<b>2</b> events since 2026"));
        assert!(html.contains(
            r#"<tr data-y="2026"><th scope="row">2026</th><td data-g="acx">1</td><td data-g="ea">2</td><td data-g="philosophia" hidden>0</td>"#
        ));
        assert!(html.contains("busiest month so far was Sep 2026, with 2"));
        assert!(html.contains(r#"[[2026,8,5,["acx","ea"],26,null],[2026,8,5,["ea"],26,null]]"#));
    }

    #[test]
    fn groups_that_start_off_are_not_counted_until_switched_on() {
        let mut reading = ev("Leviathan by Hobbes", 0, "Philosophia");
        reading.groups = vec!["philosophia".into()];
        let html = render(&[&reading, &ev("Dinner", 0, "B")]);
        assert!(html.contains("<b>1</b> events since 2026"));
        // Still in the data, so switching Philosophia on counts it.
        assert!(html.contains(r#"["philosophia"]"#));
    }

    #[test]
    fn unannounced_events_count_and_say_so() {
        let mut dinner = ev("Community dinner", 0, "");
        dinner.groups = vec!["acx".into()];
        dinner.chat = true;
        let html = render(&[&dinner]);
        assert!(html.contains("<b>1</b> events since 2026"));
        assert!(html.contains("included since November 2025"));
    }
}
