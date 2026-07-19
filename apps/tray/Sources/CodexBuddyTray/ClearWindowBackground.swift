import AppKit
import SwiftUI

/// `MenuBarExtra(.window)` hands us an opaque, square-cornered `NSWindow`. Our SwiftUI content
/// clips itself to a rounded rect, but without this the window's own opaque background still
/// shows through square behind it, so the rounding never actually reads on screen.
struct ClearWindowBackground: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        let view = NSView()
        DispatchQueue.main.async {
            guard let window = view.window else { return }
            window.backgroundColor = .clear
            window.isOpaque = false
            window.hasShadow = true
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {}
}
