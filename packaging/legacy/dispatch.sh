#!/bin/sh
set -eu
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
os=$(uname -s)
arch=$(uname -m)
case "$os" in Darwin|Linux) ;; *) echo "unsupported system" >&2; exit 2 ;; esac
case "$arch" in arm64|aarch64) target=arm64 ;; x86_64|amd64) target=x64 ;; *) echo "unsupported architecture" >&2; exit 2 ;; esac
if [ "$os" = Darwin ] && [ "$target" = x64 ] && command -v sysctl >/dev/null 2>&1 && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = 1 ]; then target=arm64; fi
exec "$base/$target/purchasely" "$@"
