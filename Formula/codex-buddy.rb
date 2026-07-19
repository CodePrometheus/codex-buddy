class CodexBuddy < Formula
  desc "Switch between and run multiple Codex CLI accounts in parallel, without forced re-logins."
  homepage "https://github.com/CodePrometheus/codex-buddy"
  version "0.1.0"
  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/CodePrometheus/codex-buddy/releases/download/v0.1.0/codex-buddy-aarch64-apple-darwin.tar.xz"
      sha256 "79f12d6c3c31fffd9adcbe5b465711774d57314c2c6f5d88adcc733764470d7c"
    end
    if Hardware::CPU.intel?
      url "https://github.com/CodePrometheus/codex-buddy/releases/download/v0.1.0/codex-buddy-x86_64-apple-darwin.tar.xz"
      sha256 "2526d3d09a50f11845a23a6eaff1dd307a37e5395910a28f5fe82e37dcf6c1df"
    end
  end
  license "MIT"

  BINARY_ALIASES = {
    "aarch64-apple-darwin": {},
    "x86_64-apple-darwin":  {},
  }.freeze

  def target_triple
    cpu = Hardware::CPU.arm? ? "aarch64" : "x86_64"
    os = OS.mac? ? "apple-darwin" : "unknown-linux-gnu"

    "#{cpu}-#{os}"
  end

  def install_binary_aliases!
    BINARY_ALIASES[target_triple.to_sym].each do |source, dests|
      dests.each do |dest|
        bin.install_symlink bin/source.to_s => dest
      end
    end
  end

  def install
    bin.install "codex-buddy" if OS.mac? && Hardware::CPU.arm?
    bin.install "codex-buddy" if OS.mac? && Hardware::CPU.intel?

    install_binary_aliases!

    # Homebrew will automatically install these, so we don't need to do that
    doc_files = Dir["README.*", "readme.*", "LICENSE", "LICENSE.*", "CHANGELOG.*"]
    leftover_contents = Dir["*"] - doc_files

    # Install any leftover files in pkgshare; these are probably config or
    # sample files.
    pkgshare.install(*leftover_contents) unless leftover_contents.empty?
  end
end
