class Tovuk < Formula
  desc "Use Tovuk scraper APIs from a native CLI"
  homepage "https://tovuk.com"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/tovuk/tovuk/releases/download/v0.1.118/tovuk-0.1.118-aarch64-apple-darwin",
          using: :nounzip
      sha256 "e59c4e827b1b036e401e48d3b252e3e48ec40779525c19a2ef241cb216549d1b"
    end
    on_intel do
      url "https://github.com/tovuk/tovuk/releases/download/v0.1.118/tovuk-0.1.118-x86_64-apple-darwin",
          using: :nounzip
      sha256 "05754d5b2c37287bd221be8ff0712678a19fe6c2b36e47e52a3bf96aea61ca9c"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/tovuk/tovuk/releases/download/v0.1.118/tovuk-0.1.118-aarch64-unknown-linux-gnu",
          using: :nounzip
      sha256 "ea10bd416effb6a229a718f9ff7bcd8f409e51b2293207dcc2400613ec34a9bc"
    end
    on_intel do
      url "https://github.com/tovuk/tovuk/releases/download/v0.1.118/tovuk-0.1.118-x86_64-unknown-linux-gnu",
          using: :nounzip
      sha256 "eea5e48528b1f82489b1089d34839f766709fe56e406902df9177b05566eb2e5"
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
