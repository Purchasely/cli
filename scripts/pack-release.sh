#!/bin/sh
# Build the release files in <outdir>, the folder that Cloudflare Pages serves:
#   the site, reference.html, purchasely-<ver>.tgz, install.sh, SHA256SUMS, latest.json, _headers.
# The man page (man/purchasely.1 in the tarball) and reference.html come from `purchasely reference` of the built bundle.
# Needs `npm run build`. Used by the release workflow and test/release.test.ts.
set -eu

[ $# -eq 1 ] || { echo "usage: pack-release.sh <outdir>" >&2; exit 2; }
mkdir -p "$1"
out=$(cd "$1" && pwd)
[ -z "$(ls -A "$out")" ] || { echo "$out is not empty: pack into an empty directory" >&2; exit 2; }
cd "$(dirname "$0")/.."
# Pages deploys one snapshot: the release files own _headers.
[ ! -e site/_headers ] || { echo "site/_headers exists: the release files own _headers, remove it" >&2; exit 1; }
[ -f dist/bin/purchasely.js ] || { echo "dist/bin/purchasely.js is missing: run 'npm run build' first" >&2; exit 1; }

ver=$(node -p 'require("./package.json").version')
host=$(node -p 'require("./package.json").config.host')
work=$(mktemp -d "${TMPDIR:-/tmp}/purchasely-pack.XXXXXX")
trap 'rm -rf "$work"' EXIT
if command -v sha256sum >/dev/null 2>&1; then sum="sha256sum"; else sum="shasum -a 256"; fi

# The site without Markdown files, then the release files at its root.
cp -R site/. "$out/"
find "$out" \( -name '*.md' -o -name .DS_Store \) -type f -delete
node dist/bin/purchasely.js reference --format html >"$out/reference.html"

stage="$work/purchasely-$ver"
mkdir -p "$stage/dist/bin" "$stage/spec" "$stage/man"
cp dist/bin/purchasely.js "$stage/dist/bin/"
node dist/bin/purchasely.js reference --format man >"$stage/man/purchasely.1"
cp spec/management.json "$stage/spec/"
cp README.md "$stage/README.md"
cp LICENSE "$stage/LICENSE"
node -e '
  const p = require("./package.json");
  const min = { name: p.name, version: p.version, type: "module", private: true, config: { host: p.config.host } };
  require("fs").writeFileSync(process.argv[1], JSON.stringify(min, null, 2) + "\n");
' "$stage/package.json"
sed -e "s|^HOST=\$|HOST=$host|" install.sh >"$stage/install.sh"
chmod +x "$stage/install.sh"
COPYFILE_DISABLE=1 tar -czf "$out/purchasely-$ver.tgz" -C "$work" "purchasely-$ver"
cp "$stage/install.sh" "$out/install.sh"
(cd "$out" && $sum "purchasely-$ver.tgz" >SHA256SUMS)
hex=$(cut -d' ' -f1 "$out/SHA256SUMS")
printf '{"version":"%s","sha256":"%s"}\n' "$ver" "$hex" >"$out/latest.json"
cat >"$out/_headers" <<'HEADERS'
/install.sh
  Cache-Control: no-cache
  Content-Type: text/plain; charset=utf-8
/latest.json
  Cache-Control: no-cache
/SHA256SUMS
  Cache-Control: no-cache
HEADERS
echo "packed $out"
