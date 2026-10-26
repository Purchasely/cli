# Development

```bash
npm ci
npm run typecheck
npm test
npm run build
```

`npm run build` runs `scripts/build.mjs` (esbuild). It writes the ESM bundle `dist/bin/purchasely.js` and its metafile (`dist/bin/purchasely.meta.json`, never packed).

## Layout

- `src/bin/purchasely.ts`: the entry point. It calls `main("purchasely")` of `src/main.ts`.
- `src/run.ts`: `build(rt)` and `run(argv, deps)`. Tests call `run()`.
- `src/core/`: limiter, breaker, credentials, http, output, usage, update, reference, AI plugin.
- `src/surfaces/management/`: the spec loader, the naming rules, the request building and the help of the generated commands.
- `src/commands/`: hand-written commands.
- `spec/management.json`: the Management OpenAPI spec. The commands come from it at runtime. Do not edit it by hand: it is a copy of the spec of the API.

## Extensions

A program built on `purchasely` passes an `Extension` (`src/core/types.ts`) to `main(name, ext)`. It adds command modules after the built-in ones, the surface (credential and rate) of these commands, an optional identity in the `User-Agent`, a section in `reference`, and its own update channel. A new surface is added with `defineSurface()` of `src/core/credentials.ts`. `purchasely` itself has no extension: it knows the `management` surface only.

## Registry rule

A hand-written command may take over a generated name: register it before the surface and add `resource action` to `rt.taken`.

## Tests

- Tests call `run(argv, deps)` with a fake `fetch`, keychain, clock and prompt. Do not call the network. Do not use real time in limiter tests.
- `test/helpers.ts`: `setup()` gives the fakes. `EXT` is a small example extension (`acme`) for the tests of the extension seam.
- `docs/commands.md` is generated from the command tree: `npx vitest run test/docs.test.ts -t "commands.md is generated" -u`.
