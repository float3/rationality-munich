// Check the staged demo the way it will be served: every page has a language
// and its noindex, and every absolute path it asks for exists under /demo.
// Run after `pnpm stage`.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '../../www/demo');
const basePath = '/demo';

async function files(directory) {
  const result = [];
  for (const item of await fs.readdir(directory, { withFileTypes: true })) {
    const p = path.join(directory, item.name);
    if (item.isDirectory()) result.push(...(await files(p)));
    else result.push(p);
  }
  return result;
}

async function resolveLink(url) {
  const { pathname } = new URL(url, 'https://rationality-munich.com');
  assert.ok(
    pathname === basePath || pathname.startsWith(`${basePath}/`),
    `${pathname} escapes ${basePath}: the demo must not link to the live site by absolute path`,
  );
  const p = path.join(root, decodeURIComponent(pathname.slice(basePath.length)));
  for (const candidate of [p, path.join(p, 'index.html'), `${p}.html`]) {
    if (await fs.stat(candidate).then((s) => s.isFile(), () => false)) return candidate;
  }
  return null;
}

const pages = (await files(root)).filter((p) => p.endsWith('.html'));
assert.ok(pages.length >= 12, `Only ${pages.length} pages staged; did the build finish?`);

let links = 0;
for (const page of pages) {
  const html = await fs.readFile(page, 'utf8');
  assert.ok(html.includes('<html lang="en"'), `Missing document language in ${path.basename(page)}`);
  assert.ok(html.includes('name="robots"'), `Missing noindex in ${path.basename(page)}`);
  for (const match of html.matchAll(/\b(?:href|src)="(\/[^"<>]*)"/g)) {
    const url = match[1].replaceAll('&amp;', '&');
    assert.ok(await resolveLink(url), `Broken local asset/link ${url} in ${path.basename(page)}`);
    links++;
  }
}

console.log(`Passed ${pages.length} staged pages and ${links} internal links/assets.`);
