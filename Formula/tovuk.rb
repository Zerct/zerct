class Tovuk < Formula
  desc "Use Tovuk scraper APIs from a native CLI"
  homepage "https://tovuk.com"
  url "https://github.com/tovuk/tovuk.git", tag: "v0.1.116"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--path", "crates/tovuk", "--root", prefix
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
