# rationality-munich.com

[![CI](https://github.com/float3/rationality-munich/actions/workflows/ci.yml/badge.svg)](https://github.com/float3/rationality-munich/actions/workflows/ci.yml)

[rationality-munich.com](https://rationality-munich.com) is a hub for the EA,
ACX, LessWrong, AI safety and philosophy groups in Munich, and a calendar of
everything they announce. One program writes all of it.

| Path | What it is |
| --- | --- |
| `calendar/` | the program: fetches the events, merges them, writes the site |
| `calendar/src/demo/` | the pages at the root — the hub, the calendar, a page per event |

Nothing here is served from the repository. Every page is written by the
program on the server, once an hour, so a change goes live by bumping this
flake in float3/nixos and rebuilding.

## The calendar

`calendar/` fetches past and upcoming events every hour, merges events that
were posted in several places, and writes static files. Sources: LessWrong
and the EA Forum (GraphQL), Meetup (its GraphQL API, full history back to
2016), Luma (iCal: EA Munich's calendar, Munich AI Safety's and Rationality Munich's own two),
Philosophia's Google Calendar, the MCMP reading group's schedule page, and
weekly series kept by hand in `calendar/data/series.ics`.

Groups: LW/ACX, EA Munich, Munich AI Safety, the AI Safety Munich Student
Club and PauseAI Munich are shown by default; Philosophia and the
further Munich groups from the
[Recreational Thinking directory](https://recthink.substack.com/p/germany)
are one click away.

| URL | What |
| --- | --- |
| `/calendar` | upcoming events |
| `/calendar/past/` | every past event we know of |
| `/calendar/stats/` | events per year, month and weekday |
| `/calendar#<id>` | one event, e.g. `#2026-09-26-petrov-day-ritual-munich-multiplayer-petrov` |
| `/calendar/e/<id>.ics` | one event, for "Add to calendar" |
| `/calendar/feeds/all.ics` | subscription feed of everything (also `/calendar.ics`) |
| `/calendar/feeds/acx+ea.ics` | a feed per combination of `acx`, `ea`, `mais`, `aisafety`, `philosophia`, `pauseai` |
| `/calendar/feeds/agi.ics` | one feed for each further group (`mlphil`, `geb`, `agi`, …) |
| `/calendar/feed.xml` | Atom feed of upcoming events, for feed readers |

The pages filter by group in the browser (`?g=acx,ea`), and the subscribe link
follows the filter. Visitors' browsers only ever talk to rationality-munich.com.

The stats page charts events per month with a trend line, compares any
month onwards with the year before (`?since=2026-04`), and analyses reported
attendance. Visitors report attendance on the past page ("How many came?");
`src/bin/rationality-attendance.rs` is the small service behind
`/calendar/attend` that stores those reports (event, number, time; nothing
about the sender), and the calendar shows each event's median.

`calendar/data/` holds what no feed carries: older events worth listing, and
events organised only in the groups' chats, which count in the statistics
(see its README). Past events are also remembered in an archive on the
server, so events that drop out of a feed once they happen stay listed.
Archiving also settles the sign-up count. The sites go on taking RSVPs and
cancellations for weeks after an event, so the figure is fixed at what they
said the hour it ended, rather than drifting after the statistics have used
it.

To add a group: a key in `GROUPS` (`src/event.rs`) and a source in
`src/sources.rs`; a Meetup group is one `meetup_group(...)` line. Group keys
are part of feed URLs, so don't rename them.

```sh
cd calendar
cargo test
cargo run      # writes out/site/ and out/demo/ from the live feeds
```

## CI

Every push, and weekly in case a source changes its format or a link dies:

- **Rust**: `cargo fmt --check`, `clippy -D warnings`, `cargo test`
- **Nix**: alejandra, `nix flake check`, `nix build .#default` (what the server runs)
- **HTML**: every generated page through
  [html-validate](https://html-validate.org) (rules in `.htmlvalidate.json`),
  plus well-formed XML and calendar files, and every link between the pages,
  laid out the way nginx serves them
- **Links**: every link to another site, with [lychee](https://lychee.cli.rs),
  and that the WhatsApp invites have not been reset

## The pages

`calendar/src/demo/` writes the site itself, into `$STATE_DIRECTORY/demo/`,
from the same merged events as `/calendar`. Plain HTML, one stylesheet, no
script, light and dark.

| URL | What |
| --- | --- |
| `/` | the hub: what is on, the six groups, what is being planned, the further groups |
| `/calendar` | this month and next as week grids, then every announced event |
| `/events/<id>` | one event, with the organiser's announcement and an .ics |
| `/tools` | AnkiQuest and the Petrov and Arkhipov Day ceremonies |
| `/subscribe`, `/privacy`, `/impressum`, `/about` | the text pages |
| `404.html`, `robots.txt`, `sitemap.xml`, `.well-known/security.txt` | generated with the rest |

`/calendar/past/` and `/calendar/stats/` are still the older design, served
from `site/` alongside the feeds and the per-event `.ics` files; this design
has no filters, past events or statistics yet.

`src/demo/groups.rs` holds the group directory and `src/demo/plans.rs` the
things that are happening but have no date yet: the only text kept by hand.
Every line of it is copied from the hub; do not add to it, because nobody from
these groups has reviewed what the page says about them. The privacy policy
and Impressum are in `src/demo/mod.rs` rather than linked, since the hub's
copies go when the hub does. The stylesheet is `src/demo/style.css`, and the
photograph and sharing card are compiled into the binary beside it.

When the demo does replace the hub, the move is one constant: `AT` in
`src/demo/mod.rs` goes from `"/demo"` to `""`, and every link, canonical URL
and asset path follows. A test fails if anything spells a prefix out by hand.
Setting it back to `"/demo"` puts the whole site under that path again, which
is how it was reviewed before it went live.

Every colour in the stylesheet goes through one of eighteen role variables,
so dark mode is a second `:root` block rather than a second stylesheet. The
sitemap and `security.txt` are rebuilt hourly with everything else: the sitemap
lists the current event pages, and the `Expires` date stays a year ahead.
