# Hand-kept event data

All five files are compiled into the calendar binary.

## history.json: past events worth listing

Events that were announced publicly but that no feed we read still carries:

- the Munich ACX meetups announced on
  [acxmeetup.substack.com](https://acxmeetup.substack.com) between 2023 and
  2025, whose posts give the date only in prose. Left out on purpose: the two
  cancelled meetups (14 September 2024), a duplicate announcement of the June
  2023 meetup, and the organisers' names, phone numbers and home addresses
  some posts contained;
- the 2nd Unofficial ACX Weekend (March 2026), recapped on
  [recthink.substack.com](https://recthink.substack.com);
- a New Year's event on a personal Luma calendar.

Each entry needs `title`, `start` (RFC 3339 with offset), `groups` (keys from
`GROUPS` in `src/event.rs`), `label` and `url`; `end`, `location` and
`excerpt` are optional. These are listed on /calendar/past/ like any other
event.

## unannounced.json: past events from the group chats

Events organised only in the groups' WhatsApp chats since November 2025, read
from exported chat histories: the fortnightly ACX community dinners, Intro
Fellowship sessions, the Ideas Forum and the like. Cancelled ones are left
out, and so are casual meetups (lunch and coworking, bouldering, ...), which
are arranged in the EA community's spontaneous events chat and are not events
in this sense. Only a date, a generic title and the group are
kept: no names, no message text.

Each entry needs `title`, `start` and `groups`. They are listed on
/calendar/past/, marked "In the group chat" since there is nothing to link
to, and counted in the statistics like any other event.

## polls.json: "who's coming?" polls from the group chats

Sign-ups for events whose pages had none, or fewer: each poll's event time,
its group, and per answer the votes and how likely those voters were to come
("definitely" 1, "probably" 0.75, "maybe" or "50%" 0.5, a range its middle).
The expected number of people, rounded, goes to the group's event that day
starting nearest that time, so `start` is the start of the event the poll
was about (the 11 April dinner poll counts for that day's meetup). Polls about topics or dates, and polls for
events that were then cancelled, are left out. Only counts: the export does
not say who voted, and nothing else from the chat is kept.

## headcounts.json: how many came, by the organisers' count

Per event: its start, its group and how many came, with `"at_least": true`
where only a floor is known. Matched like the polls. An organiser's count
beats visitors' reports on /calendar/past/; a floor gives way to a report
above it. So far: the ACX Meetups Everywhere day on 11 April 2026 (at least
40) and its Estimation Game (30), every Estimation Game since (at least 10
each), the dinner with a visiting rationalist from Tokyo (4), and both EA
Munich Speaker Series talks (at least 20 each).

## ranges.json: how many usually came to a series

What the organisers know about a series as a whole rather than per event:
its groups, words every title has, the range of headcounts, and the date up
to which that holds. Each event of the series that nobody counted gets its
sign-ups kept inside the range, or the middle of it. So far: the ACX
community dinners, 5 to 12 people each, up to September 2026.
