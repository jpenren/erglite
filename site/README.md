# ErgLite landing page

Astro site for the ErgLite landing page. It is published as a GitHub Pages project page at
<https://jpenren.github.io/erglite/>.

## Development

```bash
cd site
npm install
npm run dev
```

The dev server serves the site at <http://localhost:4321/erglite/> because the production build is
configured with `base: '/erglite'` in `astro.config.mjs`.

## Build

```bash
npm run build
npm run preview
```

The static output is written to `site/dist`.

## Deployment

Pushing changes under `site/` to `main` triggers `.github/workflows/pages.yml`, which builds the site
and deploys it with GitHub Actions. In the repository settings, **Pages → Build and deployment** must
use **GitHub Actions** as the source.

If the project moves to a custom domain, update `site` and remove `base` in `astro.config.mjs`, then
update `public/robots.txt` and the absolute URLs in `src/layouts/Layout.astro`.

## Content notes

- The page is written in English and aimed at end users, not developers.
- Visual tokens come from `design/tokens.css`; Barlow and Barlow Condensed are bundled locally with
  Fontsource.
- Screenshots come from `design/screens/` and are optimized to WebP at build time.
- Download buttons resolve the latest installers from the GitHub Releases API. Without network access
  or when the API is rate-limited, they fall back to the releases page.
