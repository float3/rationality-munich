//! The tools page: what this site hosts for other groups to use.
//!
//! Descriptions are each project's own — www/tools.html for the two
//! ceremonies, AnkiQuest's README for AnkiQuest — for the same reason as the
//! group directory: better a short true sentence than a long invented one.
//!
//! The rows borrow the directory's `group-*` classes. They are layout, not
//! taxonomy: a numbered row with a description and some links.

use crate::demo::AT;
use crate::render::html_escape;

struct Tool {
    kind: &'static str,
    name: &'static str,
    what: &'static str,
    /// What there is to install or run, where that is not obvious.
    how: &'static str,
    /// The link the row leads with, then the rest.
    links: &'static [(&'static str, &'static str)],
}

const TOOLS: &[Tool] = &[
    Tool {
        kind: "SPACED REPETITION",
        name: "AnkiQuest",
        what: "XP, streaks, daily quests, friends and leaderboards for Anki. Only the timing of your reviews is sent, never the content of your cards, unless you choose to show off a hard card you finally learned.",
        // The server at ankiquest.rationality-munich.com has no DNS record
        // yet; when it does, an ("Open", …) goes first in this list.
        how: "An Anki add-on for desktop, an AnkiDroid build for Android, and a server you can run yourself",
        links: &[
            (
                "Desktop add-on",
                "https://github.com/float3/ankiquest/releases/latest",
            ),
            (
                "Android build",
                "https://github.com/float3/AnkiQuest-Android/releases/latest",
            ),
            ("Source", "https://github.com/float3/ankiquest"),
        ],
    },
    Tool {
        kind: "CEREMONY",
        name: "Petrov Day",
        what: "A Petrov Day ritual for two groups at once, say two cities: each side has one missile, a warning window, and false alarms that look exactly like real launches.",
        how: "",
        links: &[
            ("Open", "https://petrov.rationality-munich.com/"),
            ("Source", "https://github.com/float3/petrov"),
        ],
    },
    Tool {
        kind: "CEREMONY",
        name: "Arkhipov Day",
        what: "The same game for Arkhipov Day, 27 October: the day in 1962 when Vasili Arkhipov refused to launch a submarine’s nuclear torpedo.",
        how: "",
        links: &[
            ("Open", "https://arkhipov.rationality-munich.com/"),
            ("Source", "https://github.com/float3/petrov"),
        ],
    },
];

pub fn page() -> String {
    let rows: String = TOOLS
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let (first, rest) = t.links.split_first().expect("every tool has a link");
            let more: String = rest
                .iter()
                .map(|(label, url)| {
                    format!(
                        r#"<li><a href="{url}" aria-label="{label} — {name}">{label} ↗</a></li>"#,
                        url = html_escape(url),
                        label = html_escape(label),
                        name = html_escape(t.name),
                    )
                })
                .collect();
            let how = if t.how.is_empty() {
                String::new()
            } else {
                format!(r#"<div class="group-formats">{}</div>"#, html_escape(t.how))
            };
            format!(
                r#"<article class="group-row"><div class="group-number">{number:02}<span>↗</span></div><div class="group-info"><p class="eyebrow">{kind}</p><h3>{name}</h3><p>{what}</p>{how}</div><div class="group-actions"><a class="text-link" href="{url}">{label}<span aria-hidden="true">↗</span></a><ul>{more}</ul></div></article>"#,
                number = i + 1,
                kind = t.kind,
                name = html_escape(t.name),
                what = html_escape(t.what),
                url = html_escape(first.1),
                label = html_escape(first.0),
            )
        })
        .collect();
    format!(
        r##"<main id="main" class="wrap tools-page">
<div class="page-heading"><p class="eyebrow">TOOLS</p><h1>Things we <em>host.</em></h1><p>Written for this community and left running for anyone else’s. Free to use for your own group, and the source is there if you would rather run your own.</p></div>
<div class="group-list">{rows}</div>
<p class="section-note">Questions, or something that breaks: <a class="underlined" href="mailto:rationality@hilll.dev">rationality@hilll.dev</a>. The ceremonies want two groups and a little setup beforehand, so give yourself a week.</p>
<aside class="coverage-note"><h3>Running one yourself</h3><p>All three are open source and meant to be self-hosted; none of them needs anything from us to run. AnkiQuest is a single binary with a config file, and the ceremonies are one program serving both days.</p><div class="actions"><a class="text-link" href="{AT}/">Back to events ↗</a></div></aside>
</main>"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_a_row_a_link_and_a_source() {
        let html = page();
        assert_eq!(
            html.matches("<article class=\"group-row\">").count(),
            TOOLS.len()
        );
        for t in TOOLS {
            assert!(html.contains(t.name), "{} is missing", t.name);
            assert!(
                t.links.iter().any(|(label, _)| *label == "Source"),
                "{} has no source link",
                t.name
            );
        }
        assert!(!html.contains("<script"));
    }

    /// The server has no DNS record yet, so nothing here may point at it:
    /// a link that does not resolve fails the link check and the visitor.
    #[test]
    fn nothing_links_to_a_host_that_does_not_exist() {
        assert!(!page().contains("ankiquest.rationality-munich.com"));
    }
}
