# rationality-munich.com

[![CI](https://github.com/float3/rationality-munich/actions/workflows/ci.yml/badge.svg)](https://github.com/float3/rationality-munich/actions/workflows/ci.yml)

The pages behind [rationality-munich.com](https://rationality-munich.com): a hub
for the EA, ACX, LessWrong, AI safety and philosophy groups in Munich, and the
program that builds its calendar.

| Path | What it is | How it goes live |
| --- | --- | --- |
| `www/` | the hub, privacy page and Impressum | push to `master`; the server pulls within five minutes |
| `calendar/` | the Rust program behind `/calendar` | bump this flake in float3/nixos and rebuild the server |
| `calendar/src/demo/` | the redesign, served at `/demo` | the same program writes it; the same flake bump deploys it |

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
Archiving also settles the sign-up count: what the sites said the hour an
event ended is what it keeps, since they go on taking RSVPs and cancellations
for weeks afterwards and the statistics have already used the figure.

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
- **Nix**: alejandra, `nix flake check`, `nix build .#default` (what the server runs).
  The flake check includes `checks.links`: links between the pages in `www/`,
  offline, since Nix builds have no network
- **HTML**: the pages in `www/` and the generated calendar and `/demo` pages
  through [html-validate](https://html-validate.org) (rules in
  `.htmlvalidate.json`), plus well-formed XML and calendar files, and every
  link between the pages, calendar and demo included
- **Links**: every link to another site, with [lychee](https://lychee.cli.rs),
  and that the WhatsApp invites have not been reset

## Editing the pages

Plain HTML, no build step. Each page carries its own CSS and a dark mode via
`prefers-color-scheme`. Open the file in a browser to check a change.

## The /demo redesign

`/demo` is a second, much larger take on the same material, meant to replace
the hub once it is good enough: a hero, this month and next as week grids, a
first-visit guide, the group directory, and the questions newcomers ask.
Nothing links to it from the hub yet and every page is `noindex`.

It is **the same events as `/calendar`**, not a copy of them. `calendar/`
writes it in the same hourly run, from the same merged events, into
`$STATE_DIRECTORY/demo/`; there is nothing to refresh by hand and nothing to go
stale. Plain HTML, one stylesheet, no script, so it works the way the rest of
the site does.

| URL | What |
| --- | --- |
| `/demo` | the hub: what is on, the six groups, a first visit, the FAQ |
| `/demo/calendar` | this month and next as week grids, then every announced event |
| `/demo/events/<id>` | one event, with the organiser's announcement and an .ics |
| `/demo/subscribe`, `/demo/privacy`, `/demo/about` | the pages that are only words |

nginx serves `/demo/` from that directory rather than from `www/`; the
location block is in float3/nixos, beside the one for `/calendar/`.

`src/demo/groups.rs` holds the group directory and the FAQ, the only copy kept
by hand here. The stylesheet is `src/demo/demo.css`, and the photograph and
sharing card are compiled into the binary beside it.
