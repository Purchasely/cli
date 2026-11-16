#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT HUP INT TERM
cargo run --quiet -p purchasely -- reference --format markdown > "$tmp"
{
  printf '%s\n\n' '<!-- Generated from purchasely reference --format markdown. Do not edit. -->'
  cat "$tmp"
} > docs/commands.md
