//! Things the groups have said are happening but have not dated yet, so no
//! feed carries them and no calendar grid can show them. Kept by hand, from
//! the hub's "Coming up" list; delete an entry once it has a date and a
//! source, because then the calendar has it.

use crate::render::html_escape;

struct Plan {
    what: &'static str,
    when: &'static str,
    detail: &'static str,
}

const PLANS: &[Plan] = &[
    Plan {
        what: "ACX Munich weekend #4",
        when: "Probably late January",
        detail: "A weekend of meetups, talks and discussions.",
    },
    Plan {
        what: "A public evening on the Hugging Face incident",
        when: "October or early November, date and venue to follow",
        detail: "Two short talks on the July 2026 agent-swarm incident, one technical and one on governance, then a moderated discussion and a mixer. Aimed at people new to AI safety.",
    },
];

pub fn section() -> String {
    if PLANS.is_empty() {
        return String::new();
    }
    let rows: String = PLANS
        .iter()
        .map(|p| {
            format!(
                r#"<li><b>{what}</b><span class="when">{when}</span><p>{detail}</p></li>"#,
                what = html_escape(p.what),
                when = html_escape(p.when),
                detail = html_escape(p.detail),
            )
        })
        .collect();
    format!(
        r#"<section class="section wrap" id="planned"><div class="section-heading"><div><p class="eyebrow">NO DATE YET</p><h2>Also being planned</h2></div><p class="heading-aside">These are not in the calendar<br>until they have a date.</p></div><ul class="plans">{rows}</ul><p class="section-note">Casual plans — lunch, coworking, bouldering — happen in the spontaneous events chat of the EA Munich WhatsApp community.</p></section>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_plan_prints_once_and_is_escaped() {
        let html = section();
        assert_eq!(html.matches("<li>").count(), PLANS.len());
        for p in PLANS {
            assert!(html.contains(p.when), "{} lost its date", p.what);
        }
        assert!(!html.contains("<script"));
    }
}
