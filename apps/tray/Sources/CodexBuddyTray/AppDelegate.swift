import AppKit
import Combine
import SwiftUI

/// Owns the status item directly instead of `MenuBarExtra`, which can't tell left-click from
/// right-click: left opens the panel, right pops a plain system menu (About / Quit).
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    let store = AccountStore()

    private var statusItem: NSStatusItem!
    private var panelWindow: NSWindow?
    private var storeSubscription: AnyCancellable?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)

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
        button.image = NSImage(systemSymbolName: "person.crop.circle", accessibilityDescription: "codex-buddy")
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

    private func color(for remainingPercent: Double) -> NSColor {
        if remainingPercent >= 50 { return .systemGreen }
        if remainingPercent >= 20 { return .systemOrange }
        return .systemRed
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

    private func togglePanel() {
        if let window = panelWindow, window.isVisible {
            window.orderOut(nil)
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
        let hosting = NSHostingView(rootView: TrayPanel(store: store))
        let window = NSWindow(contentRect: .zero, styleMask: [.borderless], backing: .buffered, defer: false)
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
        panelWindow?.orderOut(nil)
    }
}
