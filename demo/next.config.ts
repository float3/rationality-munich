import type { NextConfig } from 'next';

const nextConfig: NextConfig = {
  // Served from rationality-munich.com/demo. tools/stage.mjs and lib/base.ts
  // both depend on this value; change all three together.
  basePath: '/demo',
  // Plain files under www/demo/, like the rest of the site.
  output: 'export',
  // Directories with an index.html, so nginx resolves /demo/calendar/ without
  // any extensionless-URL rewriting of its own.
  trailingSlash: true,
  // The default is a fresh UUID per build, which renames a directory in
  // www/demo/ every time. The staged output is committed, so keep it stable.
  generateBuildId: () => 'demo',
};

export default nextConfig;
