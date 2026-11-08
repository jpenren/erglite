import { defineConfig } from 'astro/config';
import sitemap from '@astrojs/sitemap';

// Project page: https://jpenren.github.io/erglite/
// If the site moves to a custom domain, update `site` and drop `base`.
export default defineConfig({
  site: 'https://jpenren.github.io',
  base: '/erglite',
  trailingSlash: 'ignore',
  integrations: [sitemap()],
  build: {
    inlineStylesheets: 'auto',
  },
});
