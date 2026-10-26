// The bundle of the program. `tsc --noEmit` is the typecheck; this script makes what ships.
import { mkdirSync, writeFileSync } from "node:fs";
import { build } from "esbuild";

mkdirSync("dist/bin", { recursive: true });
const { metafile } = await build({
  entryPoints: ["src/bin/purchasely.ts"],
  outfile: "dist/bin/purchasely.js",
  bundle: true,
  platform: "node",
  format: "esm",
  target: "node20",
  metafile: true,
  // commander is CommonJS and calls require("node:events").
  banner: { js: 'import { createRequire } from "node:module"; const require = createRequire(import.meta.url);' },
});
// The metafile lists the inputs of the bundle. It is never packed.
writeFileSync("dist/bin/purchasely.meta.json", JSON.stringify(metafile));
