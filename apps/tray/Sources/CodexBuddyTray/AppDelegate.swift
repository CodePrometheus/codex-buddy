import AppKit
import Combine
import SwiftUI

/// A borderless `NSWindow` refuses key status by default, so the panel never got
/// `didResignKey` (no auto-close on outside clicks) and its text fields could not take focus.
private final class PanelWindow: NSWindow {
    override var canBecomeKey: Bool { true }
}

/// Owns the status item directly instead of `MenuBarExtra`, which can't tell left-click from
/// right-click: left opens the panel, right pops a plain system menu (About / Quit).
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let store = AccountStore()
    let appearance = AppearanceController()

    private var statusItem: NSStatusItem!
    private var panelWindow: NSWindow?
    private var storeSubscription: AnyCancellable?
    private var panelClosedAt: Date?
    private lazy var statusBarIcon = Self.makeStatusBarIcon()

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Hidden mode used by the packaging script: render the app icon and quit, without ever
        // showing a status item. Keeps the icon a byproduct of the real UI code, not a separate asset.
        if let i = CommandLine.arguments.firstIndex(of: "--render-icon"),
           i + 1 < CommandLine.arguments.count {
            let ok = renderAppIconPNG(to: CommandLine.arguments[i + 1])
            exit(ok ? 0 : 1)
        }

        NSApp.setActivationPolicy(.accessory)
        appearance.apply()

        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        item.button?.target = self
        item.button?.action = #selector(statusItemClicked)
        item.button?.sendAction(on: [.leftMouseUp, .rightMouseUp])
        statusItem = item

        storeSubscription = store.objectWillChange.sink { [weak self] _ in
            DispatchQueue.main.async { self?.updateStatusItemTitle() }
        }

        store.refresh()
        store.refreshDoctor()
        updateStatusItemTitle()
    }

    private func updateStatusItemTitle() {
        guard let button = statusItem.button else { return }
        button.image = statusBarIcon
        button.imagePosition = .imageLeading

        guard let account = store.activeAccount else {
            button.attributedTitle = NSAttributedString(string: "")
            return
        }
        let title = NSMutableAttributedString(
            string: " \(account.alias)",
            attributes: [.font: NSFont.menuBarFont(ofSize: 0)]
        )
        if let usage = account.tightestUsage {
            title.append(NSAttributedString(
                string: " ·\(Int(usage.remainingPercent))%",
                attributes: [.font: NSFont.menuBarFont(ofSize: 0), .foregroundColor: color(for: usage.remainingPercent)]
            ))
        }
        button.attributedTitle = title
    }

    /// System semantic colors, not Theme's palette: the menu bar always follows the *system*
    /// appearance, which the app's appearance override can't reach.
    private func color(for remainingPercent: Double) -> NSColor {
        switch Theme.Severity(remainingPercent: remainingPercent) {
        case .plenty: .systemGreen
        case .low: .systemOrange
        case .critical: .systemRed
        }
    }

    @objc private func statusItemClicked() {
        if NSApp.currentEvent?.type == .rightMouseUp {
            showMenu()
        } else {
            togglePanel()
        }
    }

    private func showMenu() {
        guard let button = statusItem.button else { return }
        let menu = NSMenu()

        let about = NSMenuItem(title: "About codex-buddy", action: #selector(showAbout), keyEquivalent: "")
        about.target = self
        menu.addItem(about)

        menu.addItem(.separator())

        let quit = NSMenuItem(title: "Quit codex-buddy", action: #selector(quit), keyEquivalent: "q")
        quit.target = self
        menu.addItem(quit)

        menu.popUp(positioning: nil, at: NSPoint(x: 0, y: button.bounds.height + 4), in: button)
    }

    @objc private func showAbout() {
        NSApp.activate(ignoringOtherApps: true)
        NSApp.orderFrontStandardAboutPanel(options: [:])
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }

    /// Clicking the status item while the panel is open resigns key first (closing it) before
    /// the click reaches `togglePanel` — a reopen within this window is that same click.
    private static let reopenDebounce: TimeInterval = 0.25

    private func togglePanel() {
        if let window = panelWindow, window.isVisible {
            window.orderOut(nil)
            return
        }
        if let closedAt = panelClosedAt, Date().timeIntervalSince(closedAt) < Self.reopenDebounce {
            return
        }
        showPanel()
    }

    private func showPanel() {
        store.refresh()
        store.refreshDoctor()

        let window = panelWindow ?? makePanelWindow()
        let size = window.contentView?.fittingSize ?? NSSize(width: Theme.panelWidth, height: 500)
        window.setContentSize(size)

        if let button = statusItem.button, let buttonWindow = button.window {
            let buttonFrame = buttonWindow.convertToScreen(button.convert(button.bounds, to: nil))
            window.setFrameTopLeftPoint(NSPoint(x: buttonFrame.midX - size.width / 2, y: buttonFrame.minY - 6))
        }

        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
    }

    private func makePanelWindow() -> NSWindow {
        let hosting = NSHostingView(rootView: TrayPanel(store: store, appearance: appearance))
        let window = PanelWindow(contentRect: .zero, styleMask: [.borderless], backing: .buffered, defer: false)
        window.contentView = hosting
        window.isOpaque = false
        window.backgroundColor = .clear
        window.hasShadow = true
        window.level = .statusBar
        window.isReleasedWhenClosed = false
        window.collectionBehavior = [.canJoinAllSpaces, .stationary]
        NotificationCenter.default.addObserver(
            self, selector: #selector(panelResignedKey), name: NSWindow.didResignKeyNotification, object: window
        )
        panelWindow = window
        return window
    }

    @objc private func panelResignedKey() {
        // A modal (e.g. the import NSOpenPanel) taking key must not close the panel under it —
        // its result and toast would render into a hidden window.
        guard NSApp.modalWindow == nil else { return }
        panelWindow?.orderOut(nil)
        panelClosedAt = Date()
    }

    /// Renders the `>_` mark (same shape as the header wordmark's "u") as a template NSImage for
    /// the status item — AppKit tints template images to match the current menu bar appearance
    /// and highlight state, so no manual light/dark handling is needed here.
    private static func makeStatusBarIcon() -> NSImage {
        let size = NSSize(width: StatusBarMarkView.width, height: StatusBarMarkView.height)
        let renderer = ImageRenderer(content: StatusBarMarkView().frame(width: size.width, height: size.height))
        renderer.scale = 3
        let image = renderer.nsImage ?? NSImage()
        image.size = size
        image.isTemplate = true
        return image
    }
}

/// Standalone rendering of the header wordmark's `>_` mark, re-baselined to its own origin.
private struct StatusBarMarkView: View {
    /// Ink bounds in wordmark space — x 118...181.856, y 96...169.6. These include the 8pt halo
    /// the 16pt stroke adds around the chevron path; sizing to the bare path instead (as this
    /// did originally) clipped the chevron's round caps flat and stretched the mark.
    private static let originX: CGFloat = 118
    private static let originY: CGFloat = 96
    private static let inkWidth: CGFloat = 63.856
    private static let inkHeight: CGFloat = 73.6

    static let height: CGFloat = 18
    static let width = inkWidth * height / inkHeight

    var body: some View {
        ZStack {
            BuddyChevronShape()
                .offset(x: -Self.originX, y: -Self.originY)
                .stroke(Color.black, style: StrokeStyle(lineWidth: 16, lineCap: .round, lineJoin: .round))
            RoundedRectangle(cornerRadius: 4.5)
                .fill(Color.black)
                .frame(width: 17.856, height: 9)
                .position(x: 164 - Self.originX + 17.856 / 2, y: 128 - Self.originY + 9 / 2)
        }
        .frame(width: Self.inkWidth, height: Self.inkHeight)
        .scaleEffect(Self.height / Self.inkHeight)
        .frame(width: Self.width, height: Self.height)
    }
}
