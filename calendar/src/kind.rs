//! What sort of event something is, read from its title: a dinner, a talk,
//! a hike. Turnout differs a lot between them, so forecasts compare like
//! with like, and the statistics page counts them apart.

/// Checked in order, first match wins: specific phrases before general
/// ones ("Estimation Game" before games, "Dine and Discuss" as a dinner),
/// and outdoor words that also turn up in other titles ("reading picnic")
/// only after the rest. Matched as lowercase substrings, German included.
const RULES: &[(&str, &[&str])] = &[
    ("estimation", &["estimation game", "calibrat"]),
    ("weekend", &["weekend", "retreat", "unconference", " camp"]),
    ("movie", &["movie", "film", "documentary", "cinema", "kino"]),
    (
        "outdoors",
        &[
            "hike",
            "walk",
            "mountain",
            "bouldering",
            "swim",
            "bike",
            "wanderung",
        ],
    ),
    (
        "dinner",
        &[
            "dinner",
            "dine ",
            "brunch",
            "lunch",
            "potluck",
            "breakfast",
            "barbecue",
            "abendessen",
        ],
    ),
    (
        "workshop",
        &[
            "workshop",
            "hackathon",
            "authentic relating",
            "circling",
            "debugging",
            "training",
            "giving game",
            "career",
            "planning",
            "plan to be",
        ],
    ),
    (
        "talk",
        &[
            "talk",
            "speaker",
            "lecture",
            "panel",
            "presentation",
            "fireside",
            "introduction",
            "intro ",
            "vortrag",
        ],
    ),
    (
        "discussion",
        &[
            "reading",
            "book club",
            "discussion",
            "discuss",
            "paper",
            "journal club",
            "seminar",
            "fellowship",
            "forum",
            "cafminar",
            "salon",
            "debate",
            "diskussion",
            "lesekreis",
        ],
    ),
    (
        "outdoors",
        &["picnic", "lake", "isar", "garden", "park", "outdoor"],
    ),
    ("games", &["game", "clocktower", "d&d", "spieleabend"]),
    (
        "social",
        &[
            "social",
            "meetup",
            "meet-up",
            "hangout",
            "party",
            "drinks",
            "pub",
            "celebration",
            "karaoke",
            "speedfriending",
            "stammtisch",
            "soirée",
            "soiree",
            "christmas",
            "weihnacht",
        ],
    ),
];

/// Names for the statistics page, in the order it lists them.
pub const NAMES: &[(&str, &str)] = &[
    ("dinner", "Dinners"),
    ("social", "Socials"),
    ("talk", "Talks"),
    ("discussion", "Discussions and reading groups"),
    ("workshop", "Workshops"),
    ("movie", "Movie nights"),
    ("estimation", "Estimation Games"),
    ("games", "Games"),
    ("outdoors", "Outdoors"),
    ("weekend", "Weekends and retreats"),
    ("other", "Other"),
];

pub fn kind(title: &str) -> &'static str {
    let t = format!(" {} ", title.to_lowercase());
    RULES
        .iter()
        .find(|(_, words)| words.iter().any(|w| t.contains(w)))
        .map_or("other", |(k, _)| k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_sort_into_kinds() {
        for (title, want) in [
            (
                "The Estimation Game: How Well Calibrated Are Your Intuitions?",
                "estimation",
            ),
            ("Dine and Discuss: The Hugging Face Incident", "dinner"),
            (
                "Community Dinner: Natalism vs Antinatalism - Thu, August 1",
                "dinner",
            ),
            ("ACX community dinner", "dinner"),
            ("Movie night: Chernobyl", "movie"),
            ("Discussion meetup: polygenic screening", "discussion"),
            (
                "Join me for a walk with (vegan) pick-nick/lunch in the Westpark",
                "outdoors",
            ),
            ("Worldbuilding Reading Picnic", "discussion"),
            ("Authentic Relating: Body Language – Tue 15.04", "workshop"),
            ("EA Munich Speaker Series with Patrick Gruban", "talk"),
            ("Introduction to Effective Altruism", "talk"),
            ("2nd Unofficial ACX Weekend", "weekend"),
            ("Blood on the Clocktower", "games"),
            ("Effektiver Altruismus München Social Meetup", "social"),
            ("EA München Weihnachts-Meetup", "social"),
            ("Taking animals seriously?", "other"),
        ] {
            assert_eq!(kind(title), want, "{title}");
        }
    }

    #[test]
    fn every_kind_has_a_name() {
        for (k, _) in RULES {
            assert!(NAMES.iter().any(|(n, _)| n == k), "{k}");
        }
    }
}
