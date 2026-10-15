//! The group directory: the only part of the site written by hand rather
//! than read from the feeds.
//!
//! Every line here is the hub's own text for that group, unchanged. Do not
//! add to it: nobody from these groups has reviewed what this page says about
//! them. Keys are the ones in `event::GROUPS`.

use crate::event::{DEFAULT_GROUPS, Event};
use crate::render::html_escape;

/// The switches above a listing: one per group, with the number of
/// events it has.
///
/// A hidden checkbox per group, labelled by a chip. Every row carries a
/// class per group it belongs to, and `switch_css` hides all rows and then
/// shows the ones whose group is ticked, so a row held by two groups
/// appears if either is on. Nothing here runs a script; the browser
/// remembers which boxes are ticked by itself.
pub fn switches(counts: &[(&str, &str, usize)], all_on: bool) -> String {
    let boxes: String = counts
        .iter()
        .map(|(key, _, _)| {
            format!(
                r#"<input type="checkbox" class="pick" id="g-{key}"{checked}>"#,
                checked = if all_on || DEFAULT_GROUPS.contains(key) {
                    " checked"
                } else {
                    ""
                },
            )
        })
        .collect();
    let chips: String = counts
        .iter()
        .map(|(key, name, n)| {
            format!(
                r#"<label class="chip" for="g-{key}">{name}{n}</label>"#,
                name = html_escape(name),
                n = match n {
                    0 => String::new(),
                    n => format!(" <span>{n}</span>"),
                },
            )
        })
        .collect();
    format!(
        r#"{boxes}<div class="filters"><p class="eyebrow">Groups</p><div class="chips">{chips}</div></div>"#
    )
}

/// The rules the switches need. They are generated from `GROUPS` and
/// appended to the stylesheet, so the list in `event.rs` stays the only
/// list of groups.
pub fn switch_css() -> String {
    let mut css = String::from(
        ".filtered .calendar-row,.filtered .day-event,\
         .filtered .event-list,.filtered .past-year,\
         .filtered .past-month{display:none}\
         .filtered .month-grid .busy{background:var(--paper)}\
         .filtered .month-grid .busy.outside{background:var(--tint-dim)}\
         .filtered .none-on{display:block}\
         .filtered:has(.pick:checked) .none-on{display:none}",
    );
    for (key, _) in crate::event::GROUPS {
        css += &format!(
            "#g-{key}:checked~.listing .calendar-row.g-{key}{{display:grid}}\
             #g-{key}:checked~.listing .day-event.g-{key},\
             #g-{key}:checked~.listing .event-list:has(.calendar-row.g-{key}),\
             #g-{key}:checked~.listing .past-year:has(.calendar-row.g-{key}),\
             #g-{key}:checked~.listing .past-month:has(.calendar-row.g-{key}){{display:block}}\
             #g-{key}:checked~.listing .month-grid td:has(.day-event.g-{key}){{background:var(--tint)}}\
             #g-{key}:checked~.filters label[for=g-{key}]{{background:var(--ink);color:var(--on-ink);border-color:var(--ink)}}\
             #g-{key}:focus-visible~.filters label[for=g-{key}]{{outline:3px solid var(--focus);outline-offset:3px}}"
        );
    }
    // Without `:has` the switches would hide every event and show none, so
    // a browser that lacks it gets the plain list and no switches.
    css += "@supports not selector(:has(*)){.filtered .calendar-row,\
            .filtered .day-event,.filtered .event-list,.filtered .past-year,\
            .filtered .past-month{display:revert}\
            .filtered .month-grid .busy{background:var(--tint)}\
            .filters,.filtered .none-on{display:none}}";
    css
}

/// A short label for an event, from the group running it. A label only;
/// the announcement says what an event actually is.
pub fn category(e: &Event) -> &'static str {
    for key in &e.groups {
        let label = match key.as_str() {
            "acx" | "rm" => "Rationality",
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
    topic: &'static str,
    name: &'static str,
    what: &'static str,
    /// What the group runs on a schedule, from the hub's "Regular events".
    /// Empty where it has nothing regular.
    regular: &'static str,
    links: &'static [(&'static str, &'static str)],
}

/// The six the hub lists, in its order. The first link is the one the row
/// leads with.
const GROUPS: &[Group] = &[
    Group {
        topic: "RATIONALITY",
        name: "LW/ACX Munich",
        what: "LessWrong and Astral Codex Ten meetups.",
        regular: "Community dinner every second Wednesday · the Estimation Game in the last seven days of each month",
        links: &[
            (
                "LessWrong",
                "https://www.lesswrong.com/groups/EBvaNj4oAn5nkkzbJ",
            ),
            (
                "WhatsApp",
                "https://chat.whatsapp.com/JekHeDBFokxLlmceXsYhLv",
            ),
            ("Substack", "https://acxmeetup.substack.com/"),
        ],
    },
    Group {
        topic: "EFFECTIVE ALTRUISM",
        name: "EA Munich",
        what: "Socials, intro programmes, workshops and talks.",
        regular: "Community dinner monthly, near the end of the month",
        links: &[
            ("Website", "https://www.eamuenchen.de/"),
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
        topic: "AI POLICY",
        name: "PauseAI Munich",
        what: "The Munich chapter of PauseAI.",
        regular: "",
        links: &[
            (
                "Website",
                "https://www.pause-ai.de/lokalgruppen#:~:text=M%C3%BCnchen",
            ),
            (
                "WhatsApp",
                "https://chat.whatsapp.com/DJh8ulxyBshLCkypMte7HR",
            ),
            ("Email", "mailto:germany+munich@pauseai.info"),
        ],
    },
    Group {
        topic: "AI SAFETY",
        name: "Munich AI Safety",
        what: "For people working on AI safety in Munich, technical or governance, and people upskilling in it.",
        regular: "",
        links: &[
            ("Luma", "https://luma.com/munich-ai-safety"),
            (
                "WhatsApp",
                "https://chat.whatsapp.com/BAvF88QyfrDKZMKo7whRX7",
            ),
        ],
    },
    Group {
        topic: "AI SAFETY",
        name: "AI Safety Munich Student Club",
        what: "A student club for AI safety in Munich.",
        regular: "Compute verification discussion every Tuesday, 18:00, online; newcomers welcome",
        links: &[
            (
                "Info doc",
                "https://docs.google.com/document/d/1jAWHNtcdX87Iaf55VynOcs-AEjwv_GOEkOO8V_Cef_U/view",
            ),
            (
                "WhatsApp",
                "https://chat.whatsapp.com/CpL5c6Ov7PS2OKr18Kvd2m",
            ),
        ],
    },
    Group {
        topic: "PHILOSOPHY",
        name: "Philosophia Munich",
        what: "Philosophy events and discussions in Munich.",
        regular: "",
        links: &[("Website", "https://www.philosophiamunich.org/")],
    },
];

pub fn section() -> String {
    let rows: String = GROUPS
        .iter()
        .enumerate()
        .map(|(i, g)| {
            // All of them, in the hub's order. Several of these groups
            // organise everything in WhatsApp, so no link here is hidden
            // behind a dropdown.
            let links: String = g
                .links
                .iter()
                .map(|(label, url)| {
                    format!(
                        r#"<li><a href="{url}" aria-label="{label} — {name}">{label}</a></li>"#,
                        url = html_escape(url),
                        label = html_escape(label),
                        name = html_escape(g.name),
                    )
                })
                .collect();
            let regular = if g.regular.is_empty() {
                String::new()
            } else {
                format!(
                    r#"<div class="group-formats">{}</div>"#,
                    html_escape(g.regular)
                )
            };
            format!(
                r#"<article class="group-row"><div class="group-number">{number:02}</div><div class="group-info"><p class="eyebrow">{topic}</p><h3>{name}</h3><p>{what}</p>{regular}<ul class="group-links">{links}</ul></div></article>"#,
                number = i + 1,
                topic = g.topic,
                name = html_escape(g.name),
                what = html_escape(g.what),
            )
        })
        .collect();
    format!(
        r#"<section class="section wrap" id="community"><div class="section-heading"><div><p class="eyebrow">THE GROUPS</p><h2>Six groups in Munich</h2></div><p class="heading-aside">Each has its own organisers<br>and runs its own events.</p></div><div class="group-list directory">{rows}</div><p class="section-note">Events are in English; PauseAI Munich’s are sometimes in German. The groups announce their own events and this page only collects them, so check the announcement for cost, registration and access.</p></section>"#
    )
}

/// The further Munich groups the hub lists. Their events are in the calendar
/// but off by default, so they are names and links here rather than rows.
const MORE: &[(&str, &str, &str)] = &[
    (
        "Philosophy of ML reading group",
        "https://tomster.userweb.mwn.de/mlregr/",
        "at the MCMP, every couple of weeks",
    ),
    (
        "Gödel, Escher, Bach reading group",
        "https://www.meetup.com/munich-weekly-activities/",
        "through Munich Weekly Activities",
    ),
    (
        "AGI Munich",
        "https://www.meetup.com/munchen-artificial-general-intelligence-meetup-group/",
        "talks on artificial general intelligence",
    ),
    (
        "Skeptics in the Pub",
        "https://www.meetup.com/skeptics-in-the-pub-munchen/",
        "talks and discussion, in German",
    ),
    (
        "Science Club Munich",
        "https://www.meetup.com/science-club-munich/",
        "",
    ),
    (
        "Minds in Motion",
        "https://www.meetup.com/minds-in-motion-munich/",
        "Socrates Café",
    ),
    (
        "Lifelong Curious & Book Lovers",
        "https://www.meetup.com/lifelong__curious/",
        "book club",
    ),
    (
        "Silent Book Club",
        "https://www.meetup.com/silent-book-club/",
        "",
    ),
    (
        "Culture Club Munich",
        "https://www.meetup.com/culture-club-munich/",
        "",
    ),
];

pub fn more() -> String {
    let items: String = MORE
        .iter()
        .map(|(name, url, note)| {
            let note = if note.is_empty() {
                String::new()
            } else {
                format!("<span>{}</span>", html_escape(note))
            };
            format!(
                r#"<li><a href="{url}">{name}</a>{note}</li>"#,
                url = html_escape(url),
                name = html_escape(name),
            )
        })
        .collect();
    format!(
        r#"<section class="section wrap" id="more-groups"><div class="section-heading"><div><p class="eyebrow">ALSO IN MUNICH</p><h2>More groups in Munich</h2></div><p class="heading-aside">Their events are in the <a class="underlined" href="/calendar">calendar</a>,<br>switched off by default.</p></div><ul class="more-groups">{items}</ul><p class="section-note">Also: <a class="underlined" href="https://www.cas.lmu.de/de/veranstaltungen/">LMU CAS</a>, <a class="underlined" href="https://www.tedxmuenchen.com/">TEDxMünchen</a>, <a class="underlined" href="https://www.tedxtum.com/">TEDxTUM</a>, <a class="underlined" href="https://www.submuc.de/">submuc</a>. Via the <a class="underlined" href="https://recthink.substack.com/p/germany">Recreational Thinking directory</a>.</p></section>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::GROUPS as KEYS;

    #[test]
    fn every_group_the_hub_shows_has_a_row_and_a_category() {
        for key in ["acx", "ea", "mais", "aisafety", "philosophia", "pauseai"] {
            let name = KEYS.iter().find(|(k, _)| *k == key).expect("a known key").1;
            assert!(GROUPS.iter().any(|g| g.name == name), "{name} is missing");
            let mut e = crate::event::tests::ev("x", 0, "A");
            e.groups = vec![key.to_string()];
            assert_ne!(category(&e), "Community", "{key} has no category");
        }
    }

    #[test]
    fn the_directory_escapes_what_it_prints_and_says_what_language() {
        let html = section() + &more();
        assert!(!html.contains("<script"));
        assert_eq!(
            html.matches("<article class=\"group-row\">").count(),
            GROUPS.len()
        );
        // Every link to every group, and every further group, each once.
        let links: usize = GROUPS.iter().map(|g| g.links.len()).sum();
        assert_eq!(html.matches("<li>").count(), links + MORE.len());
        // Nothing a group is reached through may be hidden behind a dropdown.
        assert!(!section().contains("<details"), "a group link is hidden");
        // The language note, which is the one question this page can
        // answer from the groups' own text.
        assert!(html.contains("sometimes in German"));
    }
}
