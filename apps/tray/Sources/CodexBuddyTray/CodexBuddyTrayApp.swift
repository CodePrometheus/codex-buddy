import SwiftUI

@main
struct CodexBuddyTrayApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        // No real scene: AppDelegate owns the status item + panel window directly.
        Settings {
            EmptyView()
        }
    }
}
