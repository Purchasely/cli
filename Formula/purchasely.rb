class Purchasely < Formula
  desc "Purchasely Management CLI"
  homepage "https://cli.purchasely.io"
  version "1.0.0"
  on_macos do
    on_arm do
      url "https://github.com/Purchasely/cli/releases/download/v1.0.0/purchasely-1.0.0-darwin-arm64.tar.gz"
      sha256 "1dcae74a651a8906fdaa9b15dc356aa4ff961a0d3f56536b2323b0eb28b8a8de"
    end
    on_intel do
      url "https://github.com/Purchasely/cli/releases/download/v1.0.0/purchasely-1.0.0-darwin-x64.tar.gz"
      sha256 "2daed4588ab869027df14095a87a1a166d0f427520589d47dbc799b1fcaf6914"
    end
  end
  on_linux do
    on_arm do
      url "https://github.com/Purchasely/cli/releases/download/v1.0.0/purchasely-1.0.0-linux-arm64.tar.gz"
      sha256 "5bb729e3b27656ca35b07c05c427df2dc257208b3ce95093fb859e455d3280b3"
    end
    on_intel do
      url "https://github.com/Purchasely/cli/releases/download/v1.0.0/purchasely-1.0.0-linux-x64.tar.gz"
      sha256 "6f0a47a87ca6ccc9f665c4e9f3e6075629b95815c0494abd1ab23017a0f2a601"
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
