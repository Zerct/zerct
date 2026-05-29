class Tovuk < Formula
  desc "Deploy Rust backends and static frontends to Tovuk"
  homepage "https://tovuk.com"
  url "https://registry.npmjs.org/tovuk/-/tovuk-0.1.47.tgz"
  sha256 "bd372de0925ade4c230c8186a5c7ce25e5def84cd1f18d34d9fa2880c6355dce"
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
