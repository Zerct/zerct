class Tovuk < Formula
  desc "Use Tovuk scraper APIs from a native CLI"
  homepage "https://tovuk.com"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/tovuk/tovuk/releases/download/v0.1.117/tovuk-0.1.117-aarch64-apple-darwin",
          using: :nounzip
      sha256 "d3ce54ac228d5baa2e7acb8dd199961d2381861fde1b8ec8b61cf696785c5fc5"
    end
    on_intel do
      url "https://github.com/tovuk/tovuk/releases/download/v0.1.117/tovuk-0.1.117-x86_64-apple-darwin",
          using: :nounzip
      sha256 "2ffa983a09b510be623841aa7dc49f6e4609c110c46e296f302846cb86563ade"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/tovuk/tovuk/releases/download/v0.1.117/tovuk-0.1.117-aarch64-unknown-linux-gnu",
          using: :nounzip
      sha256 "9d4340ffca9ddbec0cad557dcfd3840d9b434b3a67298c15833aeae26bfc18b6"
    end
    on_intel do
      url "https://github.com/tovuk/tovuk/releases/download/v0.1.117/tovuk-0.1.117-x86_64-unknown-linux-gnu",
          using: :nounzip
      sha256 "94c57312e8dcf8dd073c54ad3d914d69b7d39dd86fa0da9318681ad4787005be"
    end
  end

  def install
    architecture = Hardware::CPU.arm? ? "aarch64" : "x86_64"
    platform = OS.mac? ? "apple-darwin" : "unknown-linux-gnu"
    bin.install "tovuk-#{version}-#{architecture}-#{platform}" => "tovuk"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tovuk --version")
    help = shell_output("#{bin}/tovuk --help")
    assert_match "tovuk account show", help
    refute_match "tovuk account " + "update", help
    assert_match "tovuk api-key list", help
    assert_match "tovuk api-key create", help
    assert_match "tovuk api-key revoke", help
    assert_match "tovuk pricing", help
    assert_match "tovuk scraper list", help
    assert_match "tovuk scraper health", help
    assert_match "tovuk scraper show", help
    assert_match "tovuk request create", help
    assert_match "tovuk request show", help
    assert_match "tovuk request results", help
    assert_match "tovuk usage", help
    assert_match "tovuk billing [checkout plus|checkout pro|checkout max|portal]", help
    assert_match "tovuk billing checkout plus", help
    assert_match "tovuk billing portal", help
    assert_match "tovuk support create", help
    assert_match "tovuk support list", help
    assert_match "tovuk support resolve", help
    refute_match "tovuk deploy", help
    refute_match "tovuk service", help
    refute_match "tovuk storage", help
    refute_match "tovuk sqlite", help
    refute_match "tovuk kv", help
    refute_match "tovuk queue", help
    refute_match "tovuk cron", help
    refute_match "tovuk secrets", help
  end
end
