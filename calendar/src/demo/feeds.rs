//! The subscription page at `/calendar/subscribe/`: pick the groups, get
//! the address of the feed that holds exactly those.
//!
//! `main.rs` writes one file per combination of `COMBO_GROUPS`, so the page
//! carries a link to every one of them and the stylesheet shows the one the
//! ticked boxes add up to. The rules chain `~` from the checkboxes rather
//! than asking `:has`, so the page works in any browser with sibling
//! selectors, and it runs no script. 127 links repeat a great deal, which
//! is what gzip is for.

use crate::event::{COMBO_GROUPS, GROUPS};
use crate::render::html_escape;

use super::{ARROW, groups};

const HOST: &str = "rationality-munich.com";

/// The combinations, as the bits of a number: bit `i` is `COMBO_GROUPS[i]`.
/// `main.rs` builds the file names the same way round.
fn keys(mask: u32) -> Vec<&'static str> {
    COMBO_GROUPS
        .iter()
        .enumerate()
        .filter(|(i, _)| mask & (1 << i) != 0)
        .map(|(_, k)| *k)
        .collect()
}

fn address(keys: &[&str]) -> String {
    format!("{HOST}/calendar/feeds/{}.ics", keys.join("+"))
}

/// One address, with the two ways of taking it: the link a calendar app
/// opens, and the text Google and Outlook want pasted.
fn offer(class: &str, label: &str, keys: &[&str]) -> String {
    let at = address(keys);
    format!(
        r#"<div class="feed {class}">{label}<a class="button primary" href="webcal://{at}">Open in your calendar app {ARROW}</a><code>https://{at}</code></div>"#
    )
}

pub fn page() -> String {
    let combos: String = (1..1u32 << COMBO_GROUPS.len())
        .map(|mask| offer(&format!("c{mask}"), "", &keys(mask)))
        .collect();
    // The further groups are not in the combined feeds: 2^16 files is not a
    // reasonable thing to write. They take one subscription each.
    let singles: String = GROUPS
        .iter()
        .filter(|(k, _)| !COMBO_GROUPS.contains(k))
        .map(|(key, name)| {
            offer(
                &format!("x-{key}"),
                &format!("<h2>{}</h2>", html_escape(name)),
                &[key],
            )
        })
        .collect();
    let chips = groups::switches(
        &GROUPS
            .iter()
            .map(|(key, name)| (*key, *name, 0))
            .collect::<Vec<_>>(),
        false,
    );
    format!(
        r##"<main id="main" class="wrap calendar-page">
<div class="page-heading"><p class="eyebrow">CALENDAR</p><h1>Subscribe</h1><p>Tick the groups you want. The address follows.</p></div>
<div class="picker">{chips}<div class="addresses"><p class="none">Tick a group.</p>{combos}{singles}</div></div>
<aside class="coverage-note"><h3>Where to put the address</h3><p><b>Apple Calendar, Outlook, Thunderbird:</b> the button opens them.<br><b>Google Calendar:</b> Other calendars → + → From URL.<br><b>Outlook on the web:</b> Add calendar → Subscribe from web.</p><p class="small-note">Your calendar app re-reads the address every few hours, so new events arrive on their own. Nothing is sent to this site when it does.</p><div class="actions"><a class="text-link" href="/calendar/">Upcoming events</a><a class="text-link" href="/calendar/rss.xml">RSS</a><a class="text-link" href="/calendar/feed.xml">Atom</a></div></aside>
</main>"##
    )
}

/// The rules the page needs, generated from the same list of groups, and
/// appended to the stylesheet.
pub fn css() -> String {
    let mut css = String::from(".addresses .feed{display:none}");
    // "Tick a group" stands until one of them is, whichever it is.
    for (key, _) in GROUPS {
        css += &format!("#g-{key}:checked~.addresses .none{{display:none}}");
    }
    for mask in 1..1u32 << COMBO_GROUPS.len() {
        let chain: String = COMBO_GROUPS
            .iter()
            .enumerate()
            .map(|(i, key)| {
                if mask & (1 << i) != 0 {
                    format!("#g-{key}:checked~")
                } else {
                    format!("#g-{key}:not(:checked)~")
                }
            })
            .collect();
        css += &format!("{chain}.addresses .c{mask}{{display:block}}");
    }
    for (key, _) in GROUPS.iter().filter(|(k, _)| !COMBO_GROUPS.contains(k)) {
        css += &format!("#g-{key}:checked~.addresses .x-{key}{{display:block}}");
    }
    css
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rules name the boxes in `COMBO_GROUPS` order and the markup
    /// writes them in `GROUPS` order; `~` only looks forward, so the two
    /// orders have to agree.
    #[test]
    fn the_combination_groups_come_first_and_in_order() {
        let first: Vec<&str> = GROUPS
            .iter()
            .take(COMBO_GROUPS.len())
            .map(|(k, _)| *k)
            .collect();
        assert_eq!(first, COMBO_GROUPS.to_vec());
    }

    /// Every combination needs a link and a rule, or ticking that set of
    /// groups shows nothing at all.
    #[test]
    fn every_combination_has_a_link_and_a_rule() {
        let html = page();
        let css = css();
        for mask in 1..1u32 << COMBO_GROUPS.len() {
            let at = address(&keys(mask));
            assert!(html.contains(&format!(r#"href="webcal://{at}""#)), "{at}");
            assert!(html.contains(&format!("https://{at}")), "{at}");
            assert!(
                css.contains(&format!(".addresses .c{mask}{{display:block}}")),
                "no rule for {mask}"
            );
        }
        for (key, _) in GROUPS.iter().filter(|(k, _)| !COMBO_GROUPS.contains(k)) {
            assert!(html.contains(&format!("x-{key}")), "no link for {key}");
            assert!(css.contains(&format!(".x-{key}{{display:block}}")), "{key}");
        }
    }

    /// Every address on the page has to be a file `main.rs` writes, or the
    /// button leads to a 404.
    #[test]
    fn every_address_names_a_feed_that_is_written() {
        let written: Vec<String> = crate::group_sets().iter().map(|s| address(s)).collect();
        let html = page();
        let mut seen = 0;
        for link in html.split("webcal://").skip(1) {
            let at = link.split('"').next().expect("a closed attribute");
            assert!(written.iter().any(|w| w == at), "{at} is not written");
            seen += 1;
        }
        assert_eq!(seen, written.len(), "a feed is written but not offered");
    }

    /// What the page shows when it loads: the groups that are on by
    /// default, which is the feed every other page links to.
    #[test]
    fn the_boxes_start_on_the_default_feed() {
        let ticked: Vec<&str> = GROUPS
            .iter()
            .map(|(k, _)| *k)
            .filter(|k| crate::event::DEFAULT_GROUPS.contains(k))
            .collect();
        let mask = (1..1u32 << COMBO_GROUPS.len())
            .find(|m| keys(*m) == ticked)
            .expect("the default groups are a combination");
        let html = page();
        for key in &ticked {
            assert!(
                html.contains(&format!(r#"id="g-{key}" checked"#)),
                "{key} does not start ticked"
            );
        }
        assert!(html.contains(&format!(r#"class="feed c{mask}""#)));
        assert!(!html.contains("<script"), "the page runs no script");
    }
}
