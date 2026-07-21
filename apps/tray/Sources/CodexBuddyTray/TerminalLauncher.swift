import AppKit
import Foundation

enum TerminalLauncher {
    /// Opens Terminal running `codex-buddy run <alias>` for parallel use. Writes a `.command`
    /// script rather than shelling through AppleScript, so the alias only ever needs standard
    /// single-quote shell escaping, not AppleScript string escaping too. One fixed file per
    /// alias (owner-only, in the per-user temp dir), overwritten on each launch — nothing
    /// accumulates. Returns false when the script could not be written.
    @discardableResult
    static func run(alias: String) -> Bool {
        let escaped = alias.replacingOccurrences(of: "'", with: "'\\''")
        let script = "#!/bin/sh\nexec codex-buddy run '\(escaped)'\n"
        let url = FileManager.default.temporaryDirectory
            .appendingPathComponent("codex-buddy-run-\(alias).command")
        guard (try? script.write(to: url, atomically: true, encoding: .utf8)) != nil else {
            return false
        }
        try? FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: url.path)
        NSWorkspace.shared.open(url)
        return true
    }
}
