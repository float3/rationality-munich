//! Feeds of upcoming events for feed readers, as Atom (`feed.xml`) and as
//! RSS 2.0 (`rss.xml`), which some tools and bots only read. Calendar apps
//! get the .ics feeds instead.

use chrono::{DateTime, Utc};
use chrono_tz::Europe::Berlin;

use crate::event::{DEFAULT_GROUPS, Event, group_name};
use crate::render::{BASE, html_escape};

const TITLE: &str = "Rationality Munich: upcoming events";
const SUBTITLE: &str = "Events from the rationality and EA groups in Munich";

/// When, where, who, then the excerpt.
fn summary(e: &Event) -> String {
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
    summary
}

/// The default groups' upcoming events. Each entry's `updated` is its start
/// time rather than now, so readers don't mark every event new every hour.
pub fn atom(upcoming: &[&Event], now: DateTime<Utc>) -> String {
    let entries: String = upcoming
        .iter()
        .filter(|e| e.in_any(DEFAULT_GROUPS))
        .map(|e| {
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
                summary = html_escape(&summary(e)),
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <id>{BASE}/feed.xml</id>
  <title>{TITLE}</title>
  <subtitle>{SUBTITLE}</subtitle>
  <link rel="self" href="{BASE}/feed.xml"/>
  <link href="{BASE}"/>
  <updated>{now}</updated>
  <author><name>Rationality Munich</name></author>
{entries}</feed>
"#,
        now = now.to_rfc3339(),
    )
}

/// The same events as RSS 2.0. Items carry no `pubDate`: RSS means the date
/// something was published, which is not known, and a future date makes
/// some readers hide the item. Readers use the time they first saw it.
pub fn rss(upcoming: &[&Event], now: DateTime<Utc>) -> String {
    let items: String = upcoming
        .iter()
        .filter(|e| e.in_any(DEFAULT_GROUPS))
        .map(|e| {
            format!(
                r#"    <item>
      <title>{title}</title>
      <link>{BASE}#{id}</link>
      <guid isPermaLink="true">{BASE}#{id}</guid>
      <description>{summary}</description>
    </item>
"#,
                id = e.id,
                title = html_escape(&e.title),
                summary = html_escape(&summary(e)),
            )
        })
        .collect();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>{TITLE}</title>
    <link>{BASE}</link>
    <description>{SUBTITLE}</description>
    <language>en</language>
    <atom:link rel="self" type="application/rss+xml" href="{BASE}/rss.xml"/>
    <lastBuildDate>{now}</lastBuildDate>
{items}  </channel>
</rss>
"#,
        now = now.to_rfc2822(),
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

        let rss = rss(&events.iter().collect::<Vec<_>>(), Utc::now());
        assert!(rss.contains("<title>Dinner &amp; &lt;talk&gt;</title>"));
        assert!(rss.contains(
            r#"<guid isPermaLink="true">https://rationality-munich.com/calendar#2026-09-26-dinner-talk</guid>"#
        ));
        assert!(!rss.contains("Leviathan") && !rss.contains("<pubDate>"));
    }
}
