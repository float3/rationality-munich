//! The tools page: what this site hosts for other groups to use.
//!
//! Each description is the project's own: www/tools.html for the
//! ceremony, and each repository's README for the rest. The same program
//! also serves an Arkhipov Day game; that one is an easter egg and is not
//! listed.
//!
//! The rows reuse the directory's `group-*` classes, which style a numbered
//! row with a description and some links.

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
        // ankiquest.rationality-munich.com has no DNS record yet. Once it
        // does, put ("Open", …) first in this list.
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
        kind: "GROUP GAME",
        name: "Estimation game",
        what: "One question with a number for an answer. Everyone sends the range they are ninety per cent sure it falls in, from their own phone. The room then discusses it and answers again, so you can see who narrowed their range, who widened it, and whether the second answers were closer to the truth.",
        how: "",
        links: &[
            ("Open", "https://estimate.rationality-munich.com/"),
            ("Source", "https://github.com/float3/estimate"),
        ],
    },
    Tool {
        kind: "GROUP GAME",
        name: "Number games",
        what: "Everyone picks a number and the rule decides who wins: closest to a third of the average, the lowest number nobody else picked, the number the most people picked, the highest number that keeps the room's total under a limit, and seven more. Nobody sees a number, the host included, until the round is revealed.",
        how: "",
        links: &[
            ("Open", "https://numbers.rationality-munich.com/"),
            ("Source", "https://github.com/float3/numbers"),
        ],
    },
    Tool {
        kind: "GROUP GAME",
        name: "Speed updating",
        what: "Everyone says where they stand on statements about ethics, minds, AI, animals and more, on a bingo card or as many as they like. Each round pairs everybody with whoever disagrees with them most, on the statement they are furthest apart on, for a few timed minutes, and then asks how much it changed their mind. Whoever moves the people they talk to most wins.",
        how: "",
        links: &[
            ("Open", "https://updating.rationality-munich.com/"),
            ("Source", "https://github.com/float3/updating"),
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
<div class="page-heading"><p class="eyebrow">TOOLS</p><h1>Things this site hosts</h1><p>Software this site runs for the community. All of it is open source: use ours, or run your own.</p></div>
<div class="group-list">{rows}</div>
<p class="section-note">Questions and bug reports: <a class="underlined" href="mailto:rationality@hilll.dev">rationality@hilll.dev</a>. The ceremony needs two groups and some setup beforehand.</p>
<aside class="coverage-note"><h3>Running one yourself</h3><p>All of these can be self-hosted, and none of them needs anything from this server. AnkiQuest is a single binary with a config file. The ceremony and the three group games are one binary each.</p><div class="actions"><a class="text-link" href="{AT}/">Back to events</a></div></aside>
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

    /// The server has no DNS record yet. A link to it would fail the link
    /// check in CI and 404 for anyone who clicked it.
    #[test]
    fn nothing_links_to_a_host_that_does_not_exist() {
        assert!(!page().contains("ankiquest.rationality-munich.com"));
    }
}
