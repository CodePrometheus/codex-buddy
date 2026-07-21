import AppKit

// Menu-bar-only app: `AppDelegate` owns the status item and panel window directly. A plain AppKit
// entry point (rather than SwiftUI's `App`) avoids a stray empty Settings window — SwiftUI's `App`
// requires a Scene, and the placeholder `Settings { EmptyView() }` scene surfaced as a blank ⌘,
// window.
//
// The process entry point runs on the main thread, so `assumeIsolated` is sound; it lets us build
// the `@MainActor` delegate without hopping actors. `delegate` lives for the whole `run()`, which
// never returns until quit.
MainActor.assumeIsolated {
    let app = NSApplication.shared
    let delegate = AppDelegate()
    app.delegate = delegate
    app.run()
}
