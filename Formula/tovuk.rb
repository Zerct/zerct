class Tovuk < Formula
  desc "Deploy Rust backends, static frontends, and fullstack apps to Tovuk"
  homepage "https://tovuk.com"
  url "https://registry.npmjs.org/tovuk/-/tovuk-0.1.50.tgz"
  sha256 "ad71f244f30b912609bac6d9112d84ed6813bf2c93f8dd1d71a1fd5aad8d8162"
  license "MIT"

  depends_on "node"

  def install
    package_root = buildpath/"package"
    source = package_root.directory? ? package_root : buildpath

    cd source do
      system "npm", "install", *std_npm_args(prefix: libexec)
    end

    (bin/"tovuk").write <<~SH
      #!/bin/sh
      exec "#{libexec}/lib/node_modules/tovuk/node_modules/.bin/tsx" "#{libexec}/lib/node_modules/tovuk/src/tovuk.ts" "$@"
    SH
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tovuk --version")
    help = shell_output("#{bin}/tovuk --help")
    assert_match "tovuk billing [checkout|portal]", help
    assert_match "tovuk support create", help
  end
end
