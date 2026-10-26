/** The repo (or release) root: two levels up from `src/core/` and from `dist/bin/`, so a bundle finds `package.json`, `spec/` and `assets/`. */
export const ROOT = new URL("../../", import.meta.url);
