class Sshelf < Formula
  desc "Fast terminal UI for managing and connecting to SSH hosts"
  homepage "https://github.com/jiiikoo/sshelf"
  url "https://github.com/jiiikoo/sshelf/archive/refs/tags/v0.2.0-jiiikoo.tar.gz"
  sha256 "f21a3d886264ff375f45b49fc9b7bde8ae3d07dba3e809fb769cf6a9037d9fa0"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "sshelf", shell_output("#{bin}/sshelf --version")
  end
end
