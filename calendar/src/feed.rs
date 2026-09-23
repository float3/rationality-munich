//! An Atom feed of upcoming events, for feed readers. Calendar apps get the
//! .ics feeds instead.

use chrono::{DateTime, Utc};
use chrono_tz::Europe::Berlin;

use crate::event::{DEFAULT_GROUPS, Event, group_name};
use crate::render::{BASE, html_escape};

/// The default groups' upcoming events. Each entry's `updated` is its start
/// time rather than now, so readers don't mark every event new every hour.
pub fn atom(upcoming: &[&Event], now: DateTime<Utc>) -> String {
    let entries: String = upcoming
        .iter()
        .filter(|e| e.in_any(DEFAULT_GROUPS))
        .map(|e| {
            let start = e.start.with_timezone(&Berlin);
            let mut line = vec![start.format("%a %-d %b %Y, %H:%M").to_string()];
            if !e.place().is_empty() {
                line.push(e.place().to_string());
            }
            line.push(
                e.groups
                    .iter()
                    .map(|g| group_name(g))
                    .collect::<Vec<_>>()
                    .join(" & "),
            );
            let mut summary = line.join(" · ");
            if !e.excerpt.is_empty() {
                summary += &format!("\n\n{}", e.excerpt);
            }
            format!(
                r#"  <entry>
    <id>{BASE}#{id}</id>
    <title>{title}</title>
    <link href="{BASE}#{id}"/>
    <updated>{updated}</updated>
    <summary>{summary}</summary>
  </entry>
"#,
                id = e.id,
                title = html_escape(&e.title),
                updated = e.start.to_rfc3339(),
                summary = html_escape(&summary),
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <id>{BASE}/feed.xml</id>
  <title>Rationality Munich: upcoming events</title>
  <subtitle>Events from the rationality and EA groups in Munich</subtitle>
  <link rel="self" href="{BASE}/feed.xml"/>
  <link href="{BASE}"/>
  <updated>{now}</updated>
  <author><name>Rationality Munich</name></author>
{entries}</feed>
"#,
        now = now.to_rfc3339(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{assign_ids, tests::ev};

    #[test]
    fn default_groups_only_and_escaped() {
        let mut shown = ev("Dinner & <talk>", 0, "A");
        let mut reading = ev("Leviathan", 60, "B");
        reading.groups = vec!["philosophia".into()];
        shown.excerpt = "Bring \"snacks\"".into();
        let mut events = vec![shown, reading];
        assign_ids(&mut events);
        let xml = atom(&events.iter().collect::<Vec<_>>(), Utc::now());
        assert!(xml.contains("<title>Dinner &amp; &lt;talk&gt;</title>"));
        assert!(xml.contains("Bring &quot;snacks&quot;"));
        assert!(xml.contains(
            r#"<link href="https://rationality-munich.com/calendar#2026-09-26-dinner-talk"/>"#
        ));
        assert!(!xml.contains("Leviathan"));
    }
}
