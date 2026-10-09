# The /demo redesign

A design proposal for rationality-munich.com, served at
[rationality-munich.com/demo](https://rationality-munich.com/demo). It is a
separate thing from the live hub in `www/`: a different, much larger take on
the same material, kept around so it can be looked at and argued about rather
than described.

Everything in it is frozen. The five events are a snapshot of the calendar on
22 September 2026, and the mailing-list signup is a demonstration that stores
what you type in the page's memory and nothing else. Every page carries
`noindex`, so the demo does not compete with the real site in search results.

| Page | What |
| --- | --- |
| `/demo` | homepage: upcoming events, the five groups, a first-visit guide, FAQ |
| `/demo/calendar` | the snapshot as a list; `?preview=empty` and `?preview=unavailable` show the quiet and broken states |
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
pnpm publish-demo   # build, stage into ../www/demo, verify
```

`pnpm build` writes `dist/client/`; `pnpm stage` copies it into `../www/demo/`,
which is what goes live when `master` is pushed; `pnpm verify` checks the event
data and that every page in the staged tree has its noindex and no broken
internal link. Commit both the source change and the staged output — `www/` is
served as it is checked in, there is no build step on the server.

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
- **The event snapshot lives in `lib/content.ts`** and the group directory in
  `lib/groups.ts`. `pnpm assets` regenerates the `.ics` downloads from the
  former.
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
