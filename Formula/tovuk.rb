class Tovuk < Formula
  desc "Deploy Rust workers, static frontends, and full-stack services to Tovuk"
  homepage "https://tovuk.com"
  url "https://github.com/tovuk/tovuk.git", tag: "v0.1.83"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--path", "crates/tovuk", "--root", prefix
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tovuk --version")
    help = shell_output("#{bin}/tovuk --help")
    assert_match "tovuk new", help
    assert_match "tovuk deploy --dry-run", help
    assert_match "tovuk pricing", help
    assert_match "tovuk service show", help
    assert_match "tovuk billing [checkout|portal]", help
    assert_match "tovuk storage list", help
    assert_match "tovuk storage upload", help
    assert_match "tovuk storage download", help
    assert_match "tovuk storage delete", help
    assert_match "tovuk kv put", help
    assert_match "tovuk kv get", help
    assert_match "tovuk queue send", help
    assert_match "tovuk billing checkout --json", help
    assert_match "tovuk support create", help
    assert_match "tovuk support list", help
    assert_match "tovuk support resolve", help
    assert_match "tovuk abuse report", help
    assert_match "tovuk abuse list", help
    assert_match "tovuk abuse list --operator", help
    assert_match "tovuk abuse appeal", help
    assert_match "tovuk abuse triage", help
    assert_match "tovuk abuse notify-owner", help
    assert_match "tovuk abuse quarantine", help
    assert_match "tovuk abuse resolve", help
    assert_match "tovuk abuse reject", help
    assert_match "tovuk abuse release", help
  end
end
