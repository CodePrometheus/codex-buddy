import AppKit
import Foundation

/// Downloads a release bundle and swaps it in for the running app. User-triggered only, from
/// the update alert. The running bundle is moved aside before the new one lands, and moved back
/// if that second step fails, so an interrupted update never leaves no app at all. URLSession
/// downloads carry no browser quarantine flag, so the swapped-in app launches without another
/// right-click → Open.
enum UpdateInstaller {
    struct InstallError: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    static var assetName: String {
        #if arch(arm64)
            "Codex-Buddy-arm64-macOS.zip"
        #else
            "Codex-Buddy-x86_64-macOS.zip"
        #endif
    }

    /// Download and install `version`, calling back on the main queue. The caller relaunches.
    static func install(version: String, completion: @escaping (Result<Void, Error>) -> Void) {
        let url = URL(
            string:
                "https://github.com/CodePrometheus/codex-buddy/releases/download/v\(version)/\(assetName)"
        )!
        URLSession.shared.downloadTask(with: url) { file, response, error in
            let outcome = Result {
                try finish(download: file, response: response, error: error, version: version)
            }
            DispatchQueue.main.async { completion(outcome) }
        }.resume()
    }

    /// Quit and start the freshly installed bundle at the same path.
    static func relaunch() {
        let path = Bundle.main.bundlePath
        let handoff = Process()
        handoff.executableURL = URL(fileURLWithPath: "/bin/sh")
        // The delay lets this instance fully exit before LaunchServices starts the new one;
        // otherwise `open` can foreground the dying process instead.
        handoff.arguments = ["-c", "sleep 0.5; /usr/bin/open \"\(path)\""]
        try? handoff.run()
        NSApp.terminate(nil)
    }

    private static func finish(
        download: URL?, response: URLResponse?, error: Error?, version: String
    ) throws {
        if let error { throw error }
        guard let download, (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw InstallError(message: "download failed: unexpected response from GitHub")
        }

        let staging = URL(fileURLWithPath: NSTemporaryDirectory())
            .appendingPathComponent("codex-buddy-update-\(version)")
        let files = FileManager.default
        try? files.removeItem(at: staging)
        try files.createDirectory(at: staging, withIntermediateDirectories: true)
        try run("/usr/bin/ditto", ["-xk", download.path, staging.path])

        // Refuse to install anything that isn't exactly the advertised version.
        let newApp = staging.appendingPathComponent("Codex Buddy.app")
        let plist = NSDictionary(contentsOf: newApp.appendingPathComponent("Contents/Info.plist"))
        guard plist?["CFBundleShortVersionString"] as? String == version else {
            throw InstallError(message: "downloaded bundle is not version \(version)")
        }

        let current = Bundle.main.bundleURL
        let aside = staging.appendingPathComponent("previous.app")
        try files.moveItem(at: current, to: aside)
        do {
            try files.moveItem(at: newApp, to: current)
        } catch {
            try? files.moveItem(at: aside, to: current)
            throw error
        }
    }

    private static func run(_ tool: String, _ arguments: [String]) throws {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: tool)
        process.arguments = arguments
        try process.run()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            throw InstallError(message: "\(tool) exited with \(process.terminationStatus)")
        }
    }
}
