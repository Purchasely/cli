class Purchasely < Formula
  desc "Purchasely Management CLI"
  homepage "https://cli.purchasely.io"
  version "0.0.0"
  # The release workflow replaces this placeholder after checking every archive URL and checksum.
  on_macos do
    on_arm do
      url "https://github.com/Purchasely/cli/releases/download/v0.0.0/purchasely-0.0.0-darwin-arm64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/Purchasely/cli/releases/download/v0.0.0/purchasely-0.0.0-darwin-x64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/Purchasely/cli/releases/download/v0.0.0/purchasely-0.0.0-linux-arm64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
    on_intel do
      url "https://github.com/Purchasely/cli/releases/download/v0.0.0/purchasely-0.0.0-linux-x64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
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
