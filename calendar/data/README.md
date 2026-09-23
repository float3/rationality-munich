# Transcribed history

`acx-substack.json` holds the Munich ACX meetups announced on
[acxmeetup.substack.com](https://acxmeetup.substack.com) between 2023 and 2025.
Those posts give the date only in prose, so they were read once and written
down here instead of being scraped every hour. It is compiled into the
calendar binary.

Left out on purpose: the two cancelled meetups (14 September 2024), a
duplicate announcement of the June 2023 meetup, and the organisers' names,
phone numbers and home addresses that some posts contained.

Each entry needs `title`, `start` (RFC 3339 with offset), `groups` (keys from
`GROUPS` in `src/event.rs`), `label` and `url`; `end`, `location` and
`excerpt` are optional.
