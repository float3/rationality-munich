// vinext 1.0.0-beta.5 prerenders `output: 'export'` routes by calling the RSC
// handler with the bare route path: no `basePath`, no trailing slash. The
// handler honours both, so it answers 404 (or 308) and static routes are
// written off as dynamic while dynamic ones fail the build outright. The demo
// needs `basePath: '/demo'` and `trailingSlash: true`, so fix up the two
// prerender requests (and the not-found probe) here before building.
//
// Idempotent, and a no-op once vinext fixes this upstream.
import fs from 'node:fs';
import path from 'node:path';

const file = path.resolve(
  import.meta.dirname,
  '..',
  'node_modules/vinext/dist/build/prerender.js',
);

const edits = [
  [
    '`http://localhost${urlPath}`',
    '`http://localhost${config.basePath ?? ""}${urlPath}${config.trailingSlash && !urlPath.endsWith("/") ? "/" : ""}`',
  ],
  [
    '`http://localhost${NOT_FOUND_SENTINEL_PATH}`',
    '`http://localhost${config.basePath ?? ""}${NOT_FOUND_SENTINEL_PATH}${config.trailingSlash ? "/" : ""}`',
  ],
];

let source = fs.readFileSync(file, 'utf8');
let changed = 0;
for (const [from, to] of edits) {
  if (source.includes(to)) continue;
  if (!source.includes(from)) {
    throw new Error(
      `patch-vinext: ${from} not found in ${file}. Did vinext change? ` +
        'Check whether the prerender bug is fixed and drop this patch.',
    );
  }
  source = source.replaceAll(from, to);
  changed += 1;
}

if (changed > 0) {
  fs.writeFileSync(file, source);
  console.log(`patch-vinext: fixed up ${changed} prerender request site(s).`);
} else {
  console.log('patch-vinext: already applied.');
}
