import AppKit
import SwiftUI

/// Light/dark override for the panel, on top of the system setting.
enum AppearanceMode: String, CaseIterable {
    case system, light, dark

    var symbolName: String {
        switch self {
        case .system: "circle.lefthalf.filled"
        case .light: "sun.max.fill"
        case .dark: "moon.fill"
        }
    }

    var title: String {
        switch self {
        case .system: "Follow system"
        case .light: "Light"
        case .dark: "Dark"
        }
    }

    /// `nil` hands control back to the system.
    var nsAppearance: NSAppearance? {
        switch self {
        case .system: nil
        case .light: NSAppearance(named: .aqua)
        case .dark: NSAppearance(named: .darkAqua)
        }
    }
}

/// Applies the override through `NSApp.appearance` rather than SwiftUI's `.preferredColorScheme`:
/// `Theme`'s colors are `NSColor(dynamicProvider:)`, which resolve against `NSAppearance.current`
/// and never see the SwiftUI environment's color scheme.
///
/// The menu bar icon is a template image drawn by the system into the menu bar, so it always
/// tracks the *system* appearance — an override here cannot reach it.
@MainActor
final class AppearanceController: ObservableObject {
    private static let defaultsKey = "appearanceMode"

    @Published var mode: AppearanceMode {
        didSet {
            UserDefaults.standard.set(mode.rawValue, forKey: Self.defaultsKey)
            apply()
        }
    }

    init() {
        let stored = UserDefaults.standard.string(forKey: Self.defaultsKey) ?? ""
        mode = AppearanceMode(rawValue: stored) ?? .system
    }

    /// Call once at launch — `didSet` does not fire for the assignment in `init`.
    func apply() {
        NSApp.appearance = mode.nsAppearance
    }
}
