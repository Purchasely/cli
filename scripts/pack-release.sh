#!/bin/sh
# Package a native release or the one-time 0.9.0 bridge for Pages and GitHub Releases.
set -eu

cd "$(dirname "$0")/.."
bridge=0
version="$(grep '^version =' crates/purchasely/Cargo.toml | head -1 | cut -d'"' -f2)"
version_set=0
native=""
count=0
while [ $# -gt 0 ]; do
  case "$1" in
    --bridge) bridge=1; shift ;;
    --version|--native|--native-release-count)
      [ $# -ge 2 ] || { echo "missing value for $1" >&2; exit 2; }
      case "$1" in
        --version) version=$2; version_set=1 ;;
        --native) native=$2 ;;
        --native-release-count) count=$2 ;;
      esac
      shift 2 ;;
    --) shift; break ;;
    -*) echo "unknown option: $1" >&2; exit 2 ;;
    *) break ;;
  esac
done
[ $# -eq 1 ] || { echo "usage: pack-release.sh [--bridge] [--version <ver>] [--native <ver>] [--native-release-count <n>] <outdir>" >&2; exit 2; }
mkdir -p "$1"
out=$(cd "$1" && pwd)
[ -z "$(ls -A "$out")" ] || { echo "$out is not empty: pack into an empty directory" >&2; exit 2; }
[ ! -e site/_headers ] || { echo "site/_headers exists: release files own _headers" >&2; exit 1; }
bridge_ver=$(sed -n 's/^BRIDGE_VERSION=//p' packaging/bridge.env)
bridge_releases=$(sed -n 's/^BRIDGE_RELEASES=//p' packaging/bridge.env)
if [ "$bridge" -eq 1 ] && [ "$version_set" -eq 0 ]; then version=$bridge_ver; fi
: "${P12B_BIN_DIR:=$PWD/target/release-bins}"
work=$(mktemp -d "${TMPDIR:-/tmp}/purchasely-pack.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM
if command -v sha256sum >/dev/null 2>&1; then sum=sha256sum; else sum='shasum -a 256'; fi

cp -R site/. "$out/"
find "$out" \( -name '*.md' -o -name .DS_Store \) -type f -delete
stage="$work/purchasely-$version"
mkdir -p "$stage/bin" "$stage/man"
cp README.md LICENSE "$stage/"
sed -e "s|^HOST=\$|HOST=$(sed -n 's/^HOST=//p' packaging/site.env)|" \
    -e "s|^BRIDGE_VERSION=\$|BRIDGE_VERSION=$(sed -n 's/^BRIDGE_VERSION=//p' packaging/bridge.env)|" install.sh >"$stage/install.sh"
chmod +x "$stage/install.sh"
cp "$stage/install.sh" "$out/install.sh"

if [ "$bridge" -eq 1 ]; then
  [ "$version" = "$(sed -n 's/^BRIDGE_VERSION=//p' packaging/bridge.env)" ] || { echo "--bridge requires the configured bridge version" >&2; exit 2; }
  mkdir -p "$stage/dist/bin"
  cp packaging/legacy/purchasely.js "$stage/dist/bin/purchasely.js"
  # The shim is CommonJS: a parent package.json with "type": "module" (~/package.json) must not change that.
  printf '{"type":"commonjs"}\n' >"$stage/package.json"
  cp packaging/legacy/dispatch.sh "$stage/bin/purchasely"
  chmod +x "$stage/bin/purchasely"
  for target in darwin-arm64 darwin-x64 linux-x64 linux-arm64; do
    [ -x "$P12B_BIN_DIR/$target/purchasely" ] || { echo "missing executable $P12B_BIN_DIR/$target/purchasely" >&2; exit 1; }
    mkdir -p "$stage/bin/$target"
    cp "$P12B_BIN_DIR/$target/purchasely" "$stage/bin/$target/purchasely"
  done
  # The bridge is the Pages snapshot while 0.9.0 is the latest: the reference page and the man page are real.
  [ -n "${P12B_REFERENCE_BIN:-}" ] && refbin=$P12B_REFERENCE_BIN || refbin="$P12B_BIN_DIR/linux-x64/purchasely"
  [ -x "$refbin" ] || { echo "missing reference binary: $refbin" >&2; exit 1; }
  PURCHASELY_DISABLE_AUTOUPDATE=1 "$refbin" reference --format html >"$out/reference.html"
  PURCHASELY_DISABLE_AUTOUPDATE=1 "$refbin" reference --format man >"$stage/man/purchasely.1"
  archive="purchasely-$version.tgz"
  COPYFILE_DISABLE=1 tar -czf "$out/$archive" -C "$work" "purchasely-$version"
  size=$(wc -c <"$out/$archive" | tr -d ' ')
  [ "$size" -le 25165824 ] || { echo "bridge archive exceeds the 24 MiB safety limit" >&2; exit 1; }
  (cd "$out" && $sum "$archive" >SHA256SUMS)
  hex=$(cut -d' ' -f1 "$out/SHA256SUMS")
  printf '{"version":"%s","sha256":"%s"}\n' "$version" "$hex" >"$out/latest.json"
else
  [ -n "${P12B_REFERENCE_BIN:-}" ] && refbin=$P12B_REFERENCE_BIN || refbin="$P12B_BIN_DIR/linux-x64/purchasely"
  [ -x "$refbin" ] || { echo "missing reference binary: $refbin" >&2; exit 1; }
  PURCHASELY_DISABLE_AUTOUPDATE=1 "$refbin" reference --format html >"$out/reference.html"
  PURCHASELY_DISABLE_AUTOUPDATE=1 "$refbin" reference --format man >"$stage/man/purchasely.1"
  for target in darwin-arm64 darwin-x64 linux-x64 linux-arm64; do
    [ -x "$P12B_BIN_DIR/$target/purchasely" ] || { echo "missing executable $P12B_BIN_DIR/$target/purchasely" >&2; exit 1; }
    target_work="$work/native-$target"
    top="$target_work/purchasely-$version"
    mkdir -p "$top/bin" "$top/man"
    cp "$P12B_BIN_DIR/$target/purchasely" "$top/bin/purchasely"
    cp "$stage/install.sh" "$top/install.sh"
    cp "$stage/man/purchasely.1" "$top/man/purchasely.1"
    cp README.md LICENSE "$top/"
    (cd "$target_work" && COPYFILE_DISABLE=1 tar -czf "$out/purchasely-$version-$target.tar.gz" "purchasely-$version")
    rm -rf "$target_work"
  done
  if [ "$count" -gt 0 ] && [ "$count" -le "$bridge_releases" ]; then
    [ -n "$native" ] || native=$version
    bridge_ver=$(sed -n 's/^BRIDGE_VERSION=//p' packaging/bridge.env)
    bridge_archive="$work/purchasely-$bridge_ver.tgz"
    [ -f "$bridge_archive" ] || cp "${P12B_BRIDGE_ARCHIVE:-$PWD/purchasely-$bridge_ver.tgz}" "$bridge_archive"
    cp "$bridge_archive" "$out/"
    (cd "$out" && $sum purchasely-"$version"-*.tar.gz "purchasely-$bridge_ver.tgz" >SHA256SUMS)
    hex=$(awk -v f="purchasely-$bridge_ver.tgz" '$2==f {print $1}' "$out/SHA256SUMS")
    printf '{"version":"%s","sha256":"%s","native":"%s"}\n' "$bridge_ver" "$hex" "$native" >"$out/latest.json"
  else
    (cd "$out" && $sum purchasely-"$version"-*.tar.gz >SHA256SUMS)
    hex=$("$sum" "$out/SHA256SUMS" | awk '{print $1}')
    printf '{"version":"%s","sha256":"%s"}\n' "$version" "$hex" >"$out/latest.json"
  fi
fi
cat >"$out/_headers" <<'HEADERS'
/install.sh
  Cache-Control: no-cache
  Content-Type: text/plain; charset=utf-8
/latest.json
  Cache-Control: no-cache
/SHA256SUMS
  Cache-Control: no-cache
HEADERS
printf 'packed %s\n' "$out"
