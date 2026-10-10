# The /demo redesign

A design proposal for rationality-munich.com, served at
[rationality-munich.com/demo](https://rationality-munich.com/demo). It is a
separate thing from the live hub in `www/`: a different, much larger take on
the same material, kept around so it can be looked at and argued about rather
than described.

Its events are a snapshot of the live calendar, taken when the demo was last
published, and the month grids show the month that snapshot falls in and the
one after. Nothing updates on its own — a static export has no idea what day
it is when someone visits — so republishing is what moves it on. The
mailing-list signup is a demonstration that keeps what you type in the page's
memory and nothing else, and every page carries `noindex`, so the demo does
not compete with the real site in search results.

| Page | What |
| --- | --- |
| `/demo` | homepage: the months as week grids, the five groups, a first-visit guide, FAQ |
| `/demo/calendar` | this month and next as week grids, then every upcoming event as a list; `?preview=empty` and `?preview=unavailable` show the quiet and broken states |
| `/demo/events/<id>` | one event, with practical details and an `.ics` download |
| `/demo/subscribe` | the mailing-list signup demonstration |
| `/demo/sample-email` | what an invitation email could look like |
| `/demo/about-this-concept` | what this is, what it would take to make it real, and the credits |
| `/demo/privacy` | what the demo does with what you type (nothing) |

## Building it

Next.js app router on [vinext](https://github.com/cloudflare/vinext), built as
a static export. Node 22+ and pnpm.

```sh
cd demo
pnpm install
pnpm publish-demo   # snapshot, build, stage into ../www/demo, verify
```

In order: `pnpm snapshot` reads the live calendar and rewrites `lib/content.ts`
and the `.ics` downloads; `pnpm build` writes `dist/client/`; `pnpm stage`
copies that into `../www/demo/`, which is what goes live when `master` is
pushed; `pnpm verify` checks the events hang together and that every page in
the staged tree has its noindex and no broken internal link. Commit both the
source change and the staged output — `www/` is served as it is checked in,
there is no build step on the server.

Publishing is therefore also how the demo stays current. Left alone it will go
on showing the month it was published in; run `pnpm publish-demo` again and it
catches up.

`pnpm dev` serves it at `http://localhost:3000/demo` while you work.

## Things worth knowing before you change it

- **`basePath` is `/demo`** (`next.config.ts`). `next/link` and the chunk URLs
  pick that up on their own; a plain `<a href>`, `<img src>` or a path in
  `metadata` does not, so those spell it out with `base` from `lib/base.ts`.
- **`tools/patch-vinext.mjs` runs before every build.** vinext 1.0.0-beta.5
  prerenders static-export routes by asking its own RSC handler for the bare
  route path, without `basePath`; the handler answers 404 and the build fails.
  The patch prefixes those two requests. Delete it once vinext fixes this.
- **`tools/stage.mjs` flattens one directory.** vinext puts the chunks in
  `dist/client/demo/_next` but leaves `public/` at the root, so staging lifts
  `demo/` a level and drops a couple of bundler leftovers.
- **`lib/content.ts` is generated** by `tools/snapshot.mjs`; do not edit it by
  hand. It takes the upcoming events from the hub's own `feed.xml` (which
  decides what is in: upcoming, the groups the hub shows by default) and the
  end times and organisers' links from `feeds/all.ics`. The category and
  format labels are read off the title and the excerpt there — decoration, not
  claims; every event page links to the announcement. The group directory in
  `lib/groups.ts` is still written by hand.
- **`SNAPSHOT` is the demo's idea of today.** It is the day the snapshot ran,
  exported from `lib/content.ts`, and it dates the pages, rings a day in the
  grid, and decides which two months the grids show.
- **`components/month-grid.tsx` draws a month** as a seven-column week grid,
  Monday first, and `currentAndNext` picks the two months both pages show. It
  is a `<table>` on purpose: the weekday headers then mean something read
  aloud. The neighbouring month's days keep the rows square but stay empty, so
  no event is listed twice, and the snapshot day is ringed where a live
  calendar would mark today. Narrow screens scroll the grid sideways rather
  than shrink it. Anything past next month is still in the calendar page's
  list, which runs to the end of what the hub knows about.
- **`public/images/munich.webp`** is a 1600px copy of a Wikimedia photograph.
  The 3.7 MB original is not in the repository; to regenerate the webp, put it
  at `public/images/munich.jpg`, `pnpm add -D sharp`, and run `pnpm assets`.
- **`pnpm lint` fails.** The shadcn components under `components/ui/` arrive
  with their own oxlint complaints. Nothing here depends on it, and CI does not
  run it.
- **CI does not look at `www/demo/`.** Its HTML and link jobs glob `www/*.html`,
  one level deep. `pnpm verify` is what stands in for them. Running
  html-validate over the staged pages reports a pile of `void-style` and
  `attr-case` complaints about how React serialises `<br/>` and `fetchPriority`;
  those are style rules, not mistakes, and `trailingSlash` plus the checks in
  `tools/verify-build.mjs` cover the things that would actually break.

Image and content credits are in [CREDITS.md](CREDITS.md).
