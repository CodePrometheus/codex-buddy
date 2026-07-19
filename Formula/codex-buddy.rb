class CodexBuddy < Formula
  desc "Switch between and run multiple Codex CLI accounts in parallel, without forced re-logins."
  homepage "https://github.com/CodePrometheus/codex-buddy"
  version "0.2.0"
  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/CodePrometheus/codex-buddy/releases/download/v0.2.0/codex-buddy-aarch64-apple-darwin.tar.xz"
      sha256 "cd0978cf0aef14a5139c9034ed902a3e08afe7c588ed0f40a140bcc60c7db08f"
    end
    if Hardware::CPU.intel?
      url "https://github.com/CodePrometheus/codex-buddy/releases/download/v0.2.0/codex-buddy-x86_64-apple-darwin.tar.xz"
      sha256 "73f1617700b27db6818c590b08039891bf80d657f9b28240cfe4fa18d3e7ebc4"
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
