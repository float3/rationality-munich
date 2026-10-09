// Copy the static export into ../www/demo, the directory nginx serves at
// rationality-munich.com/demo.
//
// Two things need straightening out first. vinext writes the hashed chunks
// under the basePath (dist/client/demo/_next) but leaves everything from
// public/ at the root, so the tree comes out half-prefixed; and the build
// drops in a few files that only mean something to a bundler or a CDN.
// Flatten the one, skip the other, and what is left is exactly what the HTML
// asks for.
import fs from 'node:fs/promises';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const from = path.join(root, 'dist/client');
const to = path.resolve(root, '../www/demo');
const basePathDir = 'demo'; // keep in step with `basePath` in next.config.ts

// Build leftovers: a bundler manifest, a CDN hint, and a robots.txt that only
// the one at the site root is allowed to have a say about.
const skip = new Set([
  '.vite',
  'vinext-client-entry-manifest.json',
  'robots.txt',
  '_headers',
  '.assetsignore',
]);

if (!(await fs.access(from).then(() => true, () => false))) {
  throw new Error('tools/stage.mjs: no dist/client. Run `pnpm build` first.');
}

await fs.rm(to, { recursive: true, force: true });
await fs.mkdir(to, { recursive: true });

for (const entry of await fs.readdir(from, { withFileTypes: true })) {
  if (skip.has(entry.name)) continue;
  // dist/client/demo/_next/… is what the HTML asks for at /demo/_next/…, and
  // www/demo already is /demo. Lift its contents a level.
  const dest = entry.name === basePathDir ? to : path.join(to, entry.name);
  await fs.cp(path.join(from, entry.name), dest, { recursive: true });
}

console.log(`Staged the demo in ${path.relative(path.resolve(root, '..'), to)}.`);
