import AppKit
import Foundation

enum TerminalLauncher {
    /// Opens Terminal running `codex-buddy run <alias>` for parallel use. Writes a throwaway
    /// `.command` script rather than shelling through AppleScript, so the alias only ever needs
    /// standard single-quote shell escaping, not AppleScript string escaping too.
    static func run(alias: String) {
        let escaped = alias.replacingOccurrences(of: "'", with: "'\\''")
        let script = "#!/bin/sh\nexec codex-buddy run '\(escaped)'\n"
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("codex-buddy-run-\(UUID().uuidString).command")
        guard (try? script.write(to: url, atomically: true, encoding: .utf8)) != nil else { return }
        try? FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
        NSWorkspace.shared.open(url)
    }
}
