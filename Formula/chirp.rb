class Chirp < Formula
  desc "Keyboard-first task manager and ping reminder for the terminal"
  homepage "https://github.com/Chessing234/PingPal"
  license "MIT"
  head "https://github.com/Chessing234/PingPal.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "chirp", shell_output("#{bin}/chirp --help")
  end
end
