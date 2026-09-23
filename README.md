# rationality-munich.com

[![CI](https://github.com/float3/rationality-munich/actions/workflows/ci.yml/badge.svg)](https://github.com/float3/rationality-munich/actions/workflows/ci.yml)

The pages behind [rationality-munich.com](https://rationality-munich.com): a hub
for the EA, ACX, LessWrong, AI safety and philosophy groups in Munich, and the
program that builds its calendar.

| Path | What it is | How it goes live |
| --- | --- | --- |
| `www/` | the hub, privacy page and Impressum | push to `master`; the server pulls within five minutes |
| `calendar/` | the Rust program behind `/calendar` | bump this flake in float3/nixos and rebuild the server |

## The calendar

`calendar/` fetches past and upcoming events every hour, merges events that
were posted in several places, and writes static files. Sources: LessWrong
and the EA Forum (GraphQL), Meetup (iCal feed and group pages), Luma (iCal),
Philosophia's Google Calendar, and the MCMP reading group's schedule page.

Groups: LW/ACX and EA Munich are shown by default; Philosophia and the
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
| `/calendar/feeds/acx+ea.ics` | a feed per combination of `acx`, `ea`, `philosophia` |
| `/calendar/feeds/agi.ics` | one feed for each further group (`mlphil`, `geb`, `agi`, …) |
| `/calendar/feed.xml` | Atom feed of upcoming events, for feed readers |

The pages filter by group in the browser (`?g=acx,ea`), and the subscribe link
follows the filter. Visitors' browsers only ever talk to rationality-munich.com.

`calendar/data/` holds what no feed carries: older events worth listing, and
events organised only in the groups' chats, which count in the statistics
(see its README). Past events are also remembered in an archive on the
server, so events that drop out of a feed once they happen stay listed.

To add a group: a key in `GROUPS` (`src/event.rs`) and a source in
`src/sources.rs`; a Meetup group is one `meetup_group(...)` line. Group keys
are part of feed URLs, so don't rename them.

```sh
cd calendar
cargo test
cargo run      # writes out/site/ from the live feeds
```

## CI

Every push, and weekly in case a source changes its format:

- **Rust**: `cargo fmt --check`, `clippy -D warnings`, `cargo test`
- **Nix**: alejandra, `nix flake check`, `nix build .#default` (what the server runs)
- **HTML**: the pages in `www/` and the generated calendar pages through
  [html-validate](https://html-validate.org) (rules in `.htmlvalidate.json`),
  plus well-formed XML and calendar files

## Editing the pages

Plain HTML, no build step. Each page carries its own CSS and a dark mode via
`prefers-color-scheme`. Open the file in a browser to check a change.
