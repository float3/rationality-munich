# Hand-kept event data

Both files are compiled into the calendar binary.

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
