#!/bin/sh
set -eu
base=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
os=$(uname -s)
arch=$(uname -m)
case "$os" in Darwin) os_name=darwin ;; Linux) os_name=linux ;; *) echo "unsupported system" >&2; exit 2 ;; esac
case "$arch" in arm64|aarch64) arch_name=arm64 ;; x86_64|amd64) arch_name=x64 ;; *) echo "unsupported architecture" >&2; exit 2 ;; esac
if [ "$os_name" = darwin ] && [ "$arch_name" = x64 ] && command -v sysctl >/dev/null 2>&1 && [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = 1 ]; then arch_name=arm64; fi
target="$os_name-$arch_name"
exec "$base/$target/purchasely" "$@"
