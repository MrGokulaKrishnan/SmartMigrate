# Smart Migrate liquid-glass system

Smart Migrate uses a dark-violet liquid-glass treatment that keeps the supplied logo’s deep, glossy, dimensional character. It is not a pale or generic glassmorphism theme.

## Shared implementation

- Tokens: `shared/design-tokens/tokens.css`
- Reusable CSS primitives: `shared/design-tokens/liquid-glass.css`
- Website application: `apps/website/styles.css`

Use `sm-glass-nav` for a sticky navigation/title surface and `sm-glass` for prominent app panels. Both provide a translucent gradient fill, top gloss, bevel border, blur, saturation, and opaque fallback for platforms that do not support `backdrop-filter`.

## Application shell rules

When the Tauri UI is implemented, apply the treatment as follows:

| Surface | Treatment |
|---|---|
| Custom title bar / primary navigation | `sm-glass-nav`, 28px blur, subtle active border after scroll or focus |
| Sidebar and command surfaces | `sm-glass`, 18px blur, quiet border |
| Dialogs and permission prompts | Strong fill with 28px blur; do not allow background controls to look active |
| Device cards | `sm-glass`; use glow only for a selected or connected device |
| Remote session toolbar | Strong fill and higher contrast; it must remain legible over arbitrary video |

## Accessibility and motion

- Glass is a surface treatment, not a replacement for contrast. Text remains `--sm-text-1` through `--sm-text-3` over the stronger fills.
- Preserve keyboard focus using `--sm-glass-border-active` plus a visible focus ring.
- Reduced-motion mode disables ambient movement and light sweeps.
- If blur is unavailable, use the defined opaque surface fallback rather than leaving controls visually transparent.
