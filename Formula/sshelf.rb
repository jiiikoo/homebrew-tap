class Sshelf < Formula
  desc "Fast terminal UI for managing and connecting to SSH hosts"
  homepage "https://github.com/jeskarja/sshelf"
  url "https://github.com/jeskarja/sshelf/archive/refs/tags/v0.2.0-jesper.tar.gz"
  sha256 "0019dfc4b32d63c1392aa264aed2253c1e0c2fb09216f8e2cc269bbfb8bb49b5"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "sshelf", shell_output("#{bin}/sshelf --version")
  end
end
