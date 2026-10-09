import tailwindcss from '@tailwindcss/postcss';
import vinext from 'vinext';
import { defineConfig } from 'vite';

// A static export (`output: 'export'` in next.config.ts): `pnpm build` writes
// dist/client/, which is copied to www/demo/ and served by nginx like the rest
// of the site. No server, no hosting platform.
export default defineConfig({
  css: { postcss: { plugins: [tailwindcss()] } },
  plugins: [vinext()],
});
