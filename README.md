# rationality-munich.com

[![CI](https://github.com/float3/rationality-munich/actions/workflows/ci.yml/badge.svg)](https://github.com/float3/rationality-munich/actions/workflows/ci.yml)

[rationality-munich.com](https://rationality-munich.com) is a hub for the EA,
ACX, LessWrong, AI safety and philosophy groups in Munich, and a calendar of
everything they announce. one program writes all of it.

| Path | What it is |
| --- | --- |
| `calendar/` | the program: fetches the events, merges them, writes the site |
| `calendar/src/demo/` | the pages at the root — the hub, the calendar, a page per event |

nothing here is served from the repository. every page is written by the
program on the server, once an hour, so a change goes live by bumping this
flake in float3/nixos and rebuilding.

## the calendar

`calendar/` fetches past and upcoming events every hour, merges events that
were posted in several places, and writes static files. sources: LessWrong
and the EA Forum (GraphQL), Meetup (its GraphQL API, full history back to
2016), Luma (iCal: EA Munich's calendar, Munich AI Safety's and Rationality Munich's own two),
Philosophia's Google Calendar, the MCMP reading group's schedule page, and
weekly series kept by hand in `calendar/data/series.ics`.

groups: LW/ACX, EA Munich, Munich AI Safety, the AI Safety Munich Student
Club, PauseAI Munich and Rationality Munich's own events are shown by
default; Philosophia and the further Munich groups from the
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
| `/calendar/feeds/acx+ea.ics` | a feed per combination of `acx`, `ea`, `mais`, `aisafety`, `philosophia`, `pauseai`, `rm` |
| `/calendar/feeds/agi.ics` | one feed for each further group (`mlphil`, `geb`, `agi`, …) |
| `/calendar/feed.xml` | Atom feed of upcoming events, for feed readers |

the pages filter by group in the browser (`?g=acx,ea`), and the subscribe link
follows the filter. visitors' browsers only ever talk to rationality-munich.com.

the stats page charts events per month with a trend line, compares any
month onwards with the year before (`?since=2026-04`), and analyses reported
attendance. visitors report attendance on the past page ("How many came?");
`src/bin/rationality-attendance.rs` is the small service behind
`/calendar/attend` that stores those reports (event, number, time; nothing
about the sender), and the calendar shows each event's median.

`calendar/data/` holds what no feed carries: older events worth listing, and
events organised only in the groups' chats, which count in the statistics
(see its README). past events are also remembered in an archive on the
server, so events that drop out of a feed once they happen stay listed.
Archiving also settles the sign-up count. the sites go on taking RSVPs and
cancellations for weeks after an event, so the figure is fixed at what they
said the hour it ended, rather than drifting after the statistics have used
it.

to add a group: a key in `GROUPS` (`src/event.rs`) and a source in
`src/sources.rs`; a Meetup group is one `meetup_group(...)` line. group keys
are part of feed URLs, so don't rename them.

```sh
cd calendar
cargo test
cargo run      # writes out/site/ and out/demo/ from the live feeds
```

## CI

every push, and weekly in case a source changes its format or a link dies:

- **rust**: `cargo fmt --check`, `clippy -D warnings`, `cargo test`
- **Nix**: alejandra, `nix flake check`, `nix build .#default` (what the server runs)
- **HTML**: every generated page through
  [html-validate](https://html-validate.org) (rules in `.htmlvalidate.json`),
  plus well-formed XML and calendar files, and every link between the pages,
  laid out the way nginx serves them
- **links**: every link to another site, with [lychee](https://lychee.cli.rs),
  and that the WhatsApp invites have not been reset

## the pages

`calendar/src/demo/` writes the site itself, into `$STATE_DIRECTORY/demo/`,
from the same merged events as `/calendar`. plain HTML, one stylesheet, no
script, light and dark.

| URL | What |
| --- | --- |
| `/` | the hub: what is on, the six groups, the further groups |
| `/calendar` | this month and next as week grids, then every announced event |
| `/events/<id>` | one event, with the organiser's announcement and an .ics |
| `/tools` | AnkiQuest, the Petrov and Arkhipov Day ceremonies, the estimation game, the number games |
| `/calendar/past` | every past event, newest first, with the headcount form |
| `/calendar/stats` | how often the groups meet and how many come, charted server-side |
| `/subscribe`, `/privacy`, `/impressum`, `/about` | the text pages |
| `404.html`, `robots.txt`, `sitemap.xml`, `.well-known/security.txt` | generated with the rest |

`site/` still holds the feeds and the per-event `.ics` files, which calendar
apps and feed readers hold links to. its own pages are no longer served.

the groups that are off by default have their own section on `/calendar`
rather than a filter: these pages run no script, and a section nobody has to
switch on is simpler than one they do. the headcount form on `/calendar/past`
posts to the attendance service, which redirects the browser back to the
event with `#r-<id>` or `#x-<id>`; both lines are in the page already, hidden
until the fragment points at one.

`src/demo/groups.rs` holds the group directory, the only text kept by hand.
every line of it is copied from the hub; do not add to it, because nobody from
these groups has reviewed what the page says about them. the privacy policy
and Impressum are in `src/demo/mod.rs` rather than linked, because the hub
they were copied from has been deleted. the stylesheet is `src/demo/style.css`, and the
photograph and sharing card are compiled into the binary beside it.

these pages replaced the earlier hand-written hub in October 2026. where
they are served is one constant: `AT` in `src/demo/mod.rs` is `""` for the
root, and every link, canonical URL and asset path is built from it. a test
fails if anything spells a prefix out by hand. setting `AT` to `"/demo"`
puts the whole site back under that path, which is how it was reviewed
before it went live.

every colour in the stylesheet goes through one of eighteen role variables,
so dark mode is a second `:root` block rather than a second stylesheet. the
sitemap and `security.txt` are rebuilt hourly with everything else: the sitemap
lists the current event pages, and the `Expires` date stays a year ahead.
