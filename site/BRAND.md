# Brand values used on cli.purchasely.io

Source: https://www.purchasely.com, read on 2026-10-22, and its stylesheet
https://cdn.prod.website-files.com/6a4d58e61b3dad85725e1c66/css/purchasely-staging.webflow.shared.fa9bd1a53.min.css
(the `:root` variables `--_swatches---*` and `--_typography---*`).

## Colors

| Name | Hex | CSS variable on purchasely.com | Use on this page |
|---|---|---|---|
| Dark green | `#002420` | `--_swatches---dark-green` (text, primary button) | text, buttons, dark section, footer |
| Deep green | `#00352f` | `--_swatches---deep-green` (button hover) | button hover |
| Black green | `#001614` | `--_swatches---black-green` | terminal blocks |
| Light grey | `#eef3e9` | `--_swatches---light-grey` (`--container--primary`) | page background, code background |
| Light green | `#ceecd0` | `--_swatches---light-green` | headings and labels on dark green, hero gradient |
| Lavender | `#bdbbff` | `--_swatches---lavender` (`--text--featured`) | hero gradient, accents on dark green only |
| Coral | `#ffb6b2` | `--_swatches---coral` | hero gradient |
| Cream | `#fff3d6` | `--_swatches---cream` (secondary button hover) | "coming soon" pill |
| White | `#ffffff` | `--_swatches---pure-white` | cards, install field |

Derived, for contrast (WCAG AA):

| Name | Hex | How | Contrast |
|---|---|---|---|
| Secondary text | `#3c5852` | dark green 75% on light grey | 6.9:1 on `#eef3e9` |
| Secondary text on dark | `#90b09b` | light green 70% on dark green | 7.0:1 on `#002420` |

The site's own `--text--secondary` (dark green at 60%, about `#5f7770`) gives 4.27:1 on light grey. That fails AA
for body text, so this page uses the 75% mix instead. Lavender on light grey is 1.59:1, so lavender is used as text
only on dark green (9.2:1). Main pairs: dark green on light grey 14.6:1, light green on dark green 13.0:1, dark green
on lavender 9.2:1, dark green on coral 9.9:1.

## Typography

| Role | Family | Weight | File (self-hosted in `assets/fonts/`) | Source URL |
|---|---|---|---|---|
| Headings | ABC Arizona Flare (`Abcarizonaflare`, fallback Georgia) | 300 | `ABCArizonaFlare-Light.woff2` | https://cdn.prod.website-files.com/6a4d58e61b3dad85725e1c66/6a4d58e61b3dad85725e1c89_ABCArizonaFlare-Light.woff2 |
| Body | Suisse Intl Book (`Suisseintl`, fallback Arial) | 400 | `SuisseIntl-Book.woff2` | https://cdn.prod.website-files.com/6a4d58e61b3dad85725e1c66/6a4d58e61b3dad85725e1ca4_SuisseIntl-Book.woff2 |
| Body bold, h3 | Suisse Intl | 700 | `SuisseIntl-Bold.woff2` | https://cdn.prod.website-files.com/6a4d58e61b3dad85725e1c66/6a4d58e61b3dad85725e1ca5_SuisseIntl-Bold.woff2 |
| Code | system monospace (`ui-monospace`, SF Mono, Menlo, Consolas) | 400 | none | purchasely.com has no monospace face |

Scale from the site: h1 5.5rem, line height 1.1, letter spacing -0.03em, uppercase, weight 300 (this page uses
`clamp(2.75rem, 7.4vw, 5.5rem)` so it fits at 375 px). h2 3rem, line height 1, -0.03em. h4 1.25rem / 1.2.
Paragraph 1rem / 1.4 / -0.01em. Large paragraph 1.25rem to 1.375rem / 1.5.

ABC Arizona Flare Medium (500) is also on the site
(`.../6a4d58e61b3dad85725e1c86_ABCArizonaFlare-Medium.woff2`). This page does not use it.

Both families are commercial fonts (Dinamo, Swiss Typefaces) under the Purchasely licence. Kevin confirmed on
2026-10-23 that the licence covers the `cli.purchasely.io` domain.

## Logo and icons

- Logo: the inline SVG `.navbar-logo-svg` in the navbar of https://www.purchasely.com (viewBox `0 0 142 24`,
  `fill="currentColor"`). Copy: `assets/purchasely-logo.svg`. The page inlines it once as a `<symbol>` so that it takes
  the text color on light and dark sections.
- Fixed-color variant, not used:
  https://cdn.prod.website-files.com/6a4d58e61b3dad85725e1c66/6a4d58e61b3dad85725e1d5f_Purchasely-Logo-Colors.svg
- Favicon (light scheme): `assets/favicon.png` from
  https://cdn.prod.website-files.com/6a4d58e61b3dad85725e1c66/6a4d5a27c3cc770ad3d4fc1a_icon.png
- Favicon (dark scheme): `assets/favicon-white.png` from
  https://cdn.prod.website-files.com/6a4d58e61b3dad85725e1c66/6a4d5a3a4fbe8bc3cdc0a6c0_icon-white.png
- Apple touch icon: `assets/apple-touch-icon.png` from
  https://cdn.prod.website-files.com/6a4d58e61b3dad85725e1c66/6a4d5a271e348865a11e94d4_icon.png

## Layout and shape

- Radii: 1rem, 1.5rem, 2rem, 2.5rem (`--_layout---radius--*`). Buttons are pills (`99rem`).
- Side padding 2.5rem (`--_layout---misc--padding-global`), 1rem on phones.
- Hero: pastel gradient of light green, lavender and coral, as on the purchasely.com hero.
- Dark sections: dark green background with a 2.5rem radius, as the site's `theme-dark` sections.

## Theme

purchasely.com has no `prefers-color-scheme` rule. Its `theme-dark` class is a section style, not a dark mode. This page
follows the site: one theme, with light grey sections and dark green sections. Only the favicon changes with the color
scheme, as on purchasely.com.
