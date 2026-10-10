//! The group directory, and the labels the design hangs off it.
//!
//! The same six groups the hub lists, with the same links; the sentence under
//! each name is longer here because the design has room for it. Keys are the
//! ones in `event::GROUPS`.

use crate::event::Event;
use crate::render::html_escape;

/// A short label for an event, from the group running it. Decoration, not a
/// claim about the event: the announcement is the one that knows.
pub fn category(e: &Event) -> &'static str {
    for key in &e.groups {
        let label = match key.as_str() {
            "acx" => "Rationality",
            "ea" => "Effective altruism",
            "mais" | "aisafety" => "AI safety",
            "pauseai" => "AI policy",
            "philosophia" => "Philosophy",
            _ => continue,
        };
        return label;
    }
    "Community"
}

struct Group {
    number: &'static str,
    topic: &'static str,
    name: &'static str,
    what: &'static str,
    formats: &'static str,
    primary: (&'static str, &'static str),
    links: &'static [(&'static str, &'static str)],
}

const GROUPS: &[Group] = &[
    Group {
        number: "01",
        topic: "RATIONALITY",
        name: "LW/ACX Munich",
        what: "Papers, essays and blog posts, discussed over dinner twice a month. The LessWrong and Astral Codex Ten community also runs a monthly estimation game with EA Munich.",
        formats: "Dinner discussions · monthly thinking games",
        primary: (
            "LessWrong group page",
            "https://www.lesswrong.com/groups/EBvaNj4oAn5nkkzbJ",
        ),
        links: &[
            (
                "WhatsApp",
                "https://chat.whatsapp.com/JekHeDBFokxLlmceXsYhLv",
            ),
            ("Substack", "https://acxmeetup.substack.com/"),
        ],
    },
    Group {
        number: "02",
        topic: "EFFECTIVE ALTRUISM",
        name: "EA Munich",
        what: "Socials, introductory programmes, workshops and talks on using evidence and careful reasoning to help others. Newcomers are welcome, including people coming on their own.",
        formats: "Students & professionals · no prior knowledge needed",
        primary: ("eamuenchen.de", "https://www.eamuenchen.de/"),
        links: &[
            (
                "EA Forum",
                "https://forum.effectivealtruism.org/groups/E8ruG2KzaNpynpGXK",
            ),
            ("Luma", "https://luma.com/eamunich"),
            (
                "Meetup",
                "https://www.meetup.com/effective-altruism-munich/",
            ),
            (
                "LessWrong",
                "https://www.lesswrong.com/groups/cavvnnKLnWeqHPAsR",
            ),
            (
                "WhatsApp",
                "https://chat.whatsapp.com/FsQAbDcpz2D1FuNydYsw13",
            ),
        ],
    },
    Group {
        number: "03",
        topic: "AI SAFETY",
        name: "Munich AI Safety",
        what: "For people working on AI safety in Munich, technical or governance, and people upskilling in it.",
        formats: "Working and upskilling · technical and governance",
        primary: ("Upcoming on Luma", "https://luma.com/munich-ai-safety"),
        links: &[(
            "WhatsApp",
            "https://chat.whatsapp.com/BAvF88QyfrDKZMKo7whRX7",
        )],
    },
    Group {
        number: "04",
        topic: "AI SAFETY",
        name: "AI Safety Munich Student Club",
        what: "Reading groups, research and workshops on compute verification and AI governance. The club’s introduction lists its activities, branch leads and membership details.",
        formats: "Student community · research · workshops",
        primary: (
            "The club’s info doc",
            "https://docs.google.com/document/d/1jAWHNtcdX87Iaf55VynOcs-AEjwv_GOEkOO8V_Cef_U/view",
        ),
        links: &[(
            "WhatsApp",
            "https://chat.whatsapp.com/CpL5c6Ov7PS2OKr18Kvd2m",
        )],
    },
    Group {
        number: "05",
        topic: "AI POLICY",
        name: "PauseAI Munich",
        what: "The Munich chapter of PauseAI, which campaigns on the risks of advanced AI. The chapter page describes the group’s approach and how to get involved.",
        formats: "Local chapter · advocacy",
        primary: (
            "pause-ai.de",
            "https://www.pause-ai.de/lokalgruppen#:~:text=M%C3%BCnchen",
        ),
        links: &[
            (
                "WhatsApp",
                "https://chat.whatsapp.com/DJh8ulxyBshLCkypMte7HR",
            ),
            ("Email the chapter", "mailto:germany+munich@pauseai.info"),
        ],
    },
    Group {
        number: "06",
        topic: "PHILOSOPHY",
        name: "Philosophia Munich",
        what: "A student-run philosophy society open to everyone, including non-students. Discussions are in English, usually after reading a paper. No academic philosophy background is required.",
        formats: "English · weekly during semester · pre-reading",
        primary: (
            "philosophiamunich.org",
            "https://www.philosophiamunich.org/",
        ),
        links: &[],
    },
];

pub fn section() -> String {
    let rows: String = GROUPS
        .iter()
        .map(|g| {
            let more = if g.links.is_empty() {
                String::new()
            } else {
                let items: String = g
                    .links
                    .iter()
                    .map(|(label, url)| {
                        format!(
                            r#"<li><a href="{url}" aria-label="{label} — {name}">{label} ↗</a></li>"#,
                            url = html_escape(url),
                            label = html_escape(label),
                            name = html_escape(g.name),
                        )
                    })
                    .collect();
                format!("<details><summary>More ways to connect</summary><ul>{items}</ul></details>")
            };
            format!(
                r#"<article class="group-row"><div class="group-number">{number}<span>↗</span></div><div class="group-info"><p class="eyebrow">{topic}</p><h3>{name}</h3><p>{what}</p><div class="group-formats">{formats}</div></div><div class="group-actions"><a class="text-link" href="{url}">{label}<span aria-hidden="true">↗</span></a>{more}</div></article>"#,
                number = g.number,
                topic = g.topic,
                name = html_escape(g.name),
                what = html_escape(g.what),
                formats = html_escape(g.formats),
                url = html_escape(g.primary.1),
                label = html_escape(g.primary.0),
            )
        })
        .collect();
    format!(
        r#"<section class="section wrap" id="community"><div class="section-heading"><div><p class="eyebrow">THE GROUPS</p><h2>Six groups in Munich</h2></div><p class="heading-aside">Each has its own organisers,<br>events and way of doing things.</p></div><div class="group-list">{rows}</div><p class="section-note">The groups announce their own events; this page only collects them. More Munich groups are <a class="underlined" href="/#more-groups">listed on the hub</a>.</p></section>"#
    )
}

/// What newcomers ask. `<details>` rather than a script: it opens either way.
const QUESTIONS: &[(&str, &str)] = &[
    (
        "Do I need to know about rationality or effective altruism?",
        "EA Munich explicitly welcomes people without prior knowledge, and the monthly estimation game needs none. Other events are more focused: Philosophia’s discussions usually expect you to have read a paper. The announcement for your event says which.",
    ),
    (
        "Can I come on my own?",
        "Yes — most people at their first one did. A short message to the host beforehand makes it easier to find the group when you arrive. Each group organises its own events.",
    ),
    (
        "Are events in English or German?",
        "Philosophia Munich holds its discussions in English. For the others it depends on who turns up: check the announcement or ask the host. An English listing does not guarantee the language in the room.",
    ),
    (
        "What does it cost, and do I need to register?",
        "Check the original announcement for registration, capacity and any costs. Some gatherings are in cafés or private homes, so the host may need to hear from you first. Food and drink arrangements vary.",
    ),
    (
        "What about accessibility or student eligibility?",
        "Ask the organiser about step-free access, seating, noise, or anything else you need. For the AI Safety Munich Student Club, its info doc says who can join.",
    ),
];

pub fn faq() -> String {
    let items: String = QUESTIONS
        .iter()
        .map(|(q, a)| {
            format!(
                r#"<details><summary class="faq-question">{q}</summary><div class="faq-answer"><p>{a}</p></div></details>"#,
                q = html_escape(q),
                a = html_escape(a),
            )
        })
        .collect();
    format!(
        r#"<section class="faq-section"><div class="wrap faq-grid"><div><p class="eyebrow">PRACTICAL QUESTIONS</p><h2>Before you<br><em>come along.</em></h2><p>The details vary between groups.</p></div><div class="faq-list">{items}</div></div></section>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::GROUPS as KEYS;

    #[test]
    fn every_group_on_the_hub_has_a_row_and_a_category() {
        // The six the hub shows by default, by name.
        for key in ["acx", "ea", "mais", "aisafety", "philosophia", "pauseai"] {
            let name = KEYS.iter().find(|(k, _)| *k == key).expect("a known key").1;
            assert!(GROUPS.iter().any(|g| g.name == name), "{name} is missing");
            let mut e = crate::event::tests::ev("x", 0, "A");
            e.groups = vec![key.to_string()];
            assert_ne!(category(&e), "Community", "{key} has no category");
        }
    }

    #[test]
    fn the_directory_and_the_faq_escape_what_they_print() {
        let html = section() + &faq();
        assert!(!html.contains("<script"));
        assert!(html.contains("Philosophia Munich"));
        assert_eq!(
            html.matches("<article class=\"group-row\">").count(),
            GROUPS.len()
        );
    }
}
