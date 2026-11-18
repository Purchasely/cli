#!/bin/sh
set -eu
[ "$#" -eq 2 ] || { echo "usage: update-formula.sh <version> <SHA256SUMS>" >&2; exit 2; }
version=$1
sums=$2
case "$version" in *[!0-9A-Za-z.-]*|'') echo "invalid version" >&2; exit 2 ;; esac
[ "$version" != "0.9.0" ] || { echo "bridge release has no Homebrew formula" >&2; exit 1; }
[ -f "$sums" ] || { echo "missing SHA256SUMS" >&2; exit 1; }
for target in darwin-arm64 darwin-x64 linux-arm64 linux-x64; do
  archive="purchasely-$version-$target.tar.gz"
  digest=$(awk -v f="$archive" '$2==f {print $1}' "$sums")
  case "$digest" in *[!0-9a-f]*|'') echo "missing checksum for $archive" >&2; exit 1 ;; esac
  [ "${#digest}" -eq 64 ] || { echo "invalid checksum for $archive" >&2; exit 1; }
done
cat <<FORMULA
class Purchasely < Formula
  desc "Purchasely Management CLI"
  homepage "https://cli.purchasely.io"
  version "$version"
  on_macos do
    on_arm do
      url "https://github.com/Purchasely/cli/releases/download/v$version/purchasely-$version-darwin-arm64.tar.gz"
      sha256 "$(awk -v f="purchasely-$version-darwin-arm64.tar.gz" '$2==f {print $1}' "$sums")"
    end
    on_intel do
      url "https://github.com/Purchasely/cli/releases/download/v$version/purchasely-$version-darwin-x64.tar.gz"
      sha256 "$(awk -v f="purchasely-$version-darwin-x64.tar.gz" '$2==f {print $1}' "$sums")"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/Purchasely/cli/releases/download/v$version/purchasely-$version-linux-arm64.tar.gz"
      sha256 "$(awk -v f="purchasely-$version-linux-arm64.tar.gz" '$2==f {print $1}' "$sums")"
    end
    on_intel do
      url "https://github.com/Purchasely/cli/releases/download/v$version/purchasely-$version-linux-x64.tar.gz"
      sha256 "$(awk -v f="purchasely-$version-linux-x64.tar.gz" '$2==f {print $1}' "$sums")"
    end
  end
  def install
    bin.install "bin/purchasely"
    man1.install "man/purchasely.1"
  end
  test do
    assert_match version.to_s, shell_output("#{bin}/purchasely --version")
  end
end
FORMULA
