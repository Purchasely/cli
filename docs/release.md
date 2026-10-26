# Release

1. Bump `version` in `package.json`, commit.
2. Tag `v<version>` (it must equal the version) and push the tag.
3. `.github/workflows/release.yml` runs on every `v*` tag, with `contents: read`:
   - `build`: `npm ci`, a check that the tag equals the `package.json` version, `npm run typecheck`, `npm test`, `npm run build`, then `sh scripts/pack-release.sh out`. It uploads `out/` as the artifact `site`.
   - `deploy`: downloads the artifact and deploys it with `npx --yes wrangler@4.148.0 pages deploy out --project-name=purchasely-cli --branch=main` to the Cloudflare Pages project `purchasely-cli` (`cli.purchasely.io` and `cli.purchasely.com`). It is skipped for a tag that has a `-`.

`scripts/pack-release.sh <outdir>` needs an empty `<outdir>`. It fails when `site/_headers` exists, because the release files own `_headers`. The output is `site/` (without `*.md` files and `.DS_Store`) plus the release files: `install.sh` (with the `HOST` line set), `latest.json` (`{"version","sha256"}`), `purchasely-<ver>.tgz` (the bundle, `spec/management.json`, `man/purchasely.1`, a minimal `package.json` and `README.md`), `SHA256SUMS` and `_headers` (no-cache for `install.sh`, `latest.json` and `SHA256SUMS`). Pages deploys a whole snapshot, so the site holds the latest version only.

The script also generates two files from the built bundle, never by hand: `man/purchasely.1` in the tarball (`reference --format man`, dated with the pack date) and `reference.html` (`purchasely reference --format html`, served at `/reference`).

The deploy needs the repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`. The host value lives in `package.json` `config.host` only.

- A failed job: fix the cause, then re-run the failed job from the Actions page. The deploy is safe to repeat.
- Rollback: tag a new patch version with the fix. Never move or delete a published tag, because the public host holds one version.
- After a release: install on a clean machine, then check one background update.
