class Zerct < Formula
  desc "Deploy Rust backends and static frontends to Zerct"
  homepage "https://zerct.com"
  url "https://registry.npmjs.org/@zerct/zerct/-/zerct-0.1.46.tgz"
  sha256 "eb64f6c5c63b58dd8660cd0eec6d304cba18694abc61eacd13080d219f441bb5"
  license "MIT"

  depends_on "node"

  def install
    package_root = buildpath/"package"
    source = package_root.directory? ? package_root : buildpath

    cd source do
      system "npm", "install", *std_npm_args(prefix: libexec)
    end

    (bin/"zerct").write <<~SH
      #!/bin/sh
      exec "#{libexec}/lib/node_modules/@zerct/zerct/node_modules/.bin/tsx" "#{libexec}/lib/node_modules/@zerct/zerct/src/zerct.ts" "$@"
    SH
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/zerct --version")
    help = shell_output("#{bin}/zerct --help")
    assert_match "zerct billing [checkout|portal]", help
    assert_match "zerct support create", help
  end
end
