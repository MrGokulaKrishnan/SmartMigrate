# Smart Migrate brand system

## Identity

- Product name: **Smart Migrate**
- Short mark: **SM**
- Systems engine: **MigRoute**
- Core idea: **Connect. Control. Migrate.**

The supplied SM logo is the primary visual reference. Preserve its black field, glossy violet dimensionality, angular/chamfered details, and bidirectional migration motif. Do not substitute a generic cloud, folder, or Wi-Fi mark.

## Canonical media asset

The source image is stored at `assets/media/smart-migrate-logo.jpg`. The web shell receives a synchronized copy at `apps/website/public/media/smart-migrate-logo.jpg`.

## Palette and typography

Shared CSS tokens live in `shared/design-tokens/tokens.css`.

- Use `--sm-600` and darker values for large fills, not thin text.
- Use `--sm-200` through `--sm-400` for foreground accents on dark surfaces.
- Use `--sm-text-1`, `--sm-text-2`, and `--sm-text-3` for readable UI copy.
- Use angular display typography for headings, a legible sans-serif for UI text, and monospace only for device IDs and codes.

## Component language

- Dark-first surfaces, quiet bevels, sparse violet glow.
- Use liquid glass for navigation, overlays, dialogs, device cards, and prominent panels: a translucent violet fill, a restrained top highlight, a visible border, and background blur. Keep enough opacity for text contrast.
- Use the 45-degree chamfer for primary actions and key dialogs.
- Keep ordinary cards restrained with an 8px radius; avoid a wall of identical floating cards.
- Motion is reactive and 120–220ms except for the optional, reduced-motion-safe startup intro.
