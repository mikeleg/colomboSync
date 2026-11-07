# Design – "Eclissi"

Reference for the look of ColomboSync. Open the HTML files in a browser: they are static, with no scripts and no external resources.

| File | What it shows |
|---|---|
| `logo.html` | The three logo ideas; **Eclissi** is the chosen one (teal crescent of light, black circle covering it: A covering B). |
| `home.html` | Home "Le tue sincronizzazioni" with the A → B side panel. |
| `anteprima-specchio.html` | Preview step in mirror mode: files to delete in red, consent checkbox. |
| `sto-copiando.html` | Run in progress: huge percentage, progress bar, stop button. |

These boards are the dark theme, drawn before the code. Where they and the app differ, **the app wins**:
- Colors and the light variant: tokens in `src/theme.css` (light first, dark under `prefers-color-scheme` and `data-theme`). Screenshots of the real app, light theme included: `docs/screenshots/`.
- Logo geometry: `src-tauri/icons/app-icon.svg` (two circles r=28 at cx=58 and cx=45, gradient `#6ff0d6 → #1fc7a8 → #0a7d6e`).
- Flow and exact Italian copy: `docs/backlog/colombo-sync/prototype.html`, never these boards.
- Later decisions (e.g. Specchio violet instead of red, issue #4): `docs/backlog/colombo-sync/scope.md`.

## Taste of the owner
Bold, high-impact, Apple-keynote style: black surfaces, teal accent and glows, very large type, rounded cards, system font (Segoe UI Variable / SF). A plain, minimal first pass was rejected as boring. For new visual work propose distinctive options first, and keep deletion/danger red and loud.
