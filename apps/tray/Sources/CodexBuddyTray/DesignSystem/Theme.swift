import AppKit
import SwiftUI

/// Visual tokens mirroring `.agents/tray-mockup.html`: warm paper neutrals, a dusty rose/plum
/// accent, and a rotating candy-pastel palette per account. Colors switch with the system
/// appearance automatically (no manual light/dark plumbing needed at call sites).
enum Theme {
    static let ink = dynamic(light: 0x2B2A27, dark: 0xECE6DB)
    static let inkMuted = dynamic(light: 0x87807A, dark: 0xA49B8D)
    static let inkFaint = dynamic(light: 0xB6AFA6, dark: 0x75695C)

    static let hairline = dynamic(light: 0x382C24, dark: 0xFFF8EB, lightAlpha: 0.09, darkAlpha: 0.09)
    static let chip = dynamic(light: 0x382C24, dark: 0xFFF8EB, lightAlpha: 0.045, darkAlpha: 0.06)
    static let chipStrong = dynamic(light: 0x382C24, dark: 0xFFF8EB, lightAlpha: 0.085, darkAlpha: 0.11)

    static let accent = dynamic(light: 0xB2596A, dark: 0xE6A6AF)
    static let accentInk = dynamic(light: 0xFFFFFF, dark: 0x3A1620)
    static let accentSoft = dynamic(light: 0xB2596A, dark: 0xE6A6AF, lightAlpha: 0.16, darkAlpha: 0.22)

    /// Wordmark-only letter color. Deliberately not `ink`: in light mode the letters run warm
    /// (berry brown) so they sit in the same family as the pink `>_` gradient below, where the
    /// neutral-cool `ink` read as hard against it.
    static let brandInk = dynamic(light: 0x4A2E36, dark: 0xECE6DB)

    /// Gradient stops for the wordmark's `>_` mark — light is a sakura pink ramp, dark warms
    /// into apricot. The two modes are different gradients, not one gradient's light/dark pair.
    static let brandMarkStart = dynamic(light: 0xD4708F, dark: 0xDE95A6)
    static let brandMarkEnd = dynamic(light: 0xF3B6C8, dark: 0xF2CB96)

    static let success = dynamic(light: 0x4FA968, dark: 0x7FD99A)
    static let warning = dynamic(light: 0xE3A23D, dark: 0xF4C876)
    static let critical = dynamic(light: 0xE2685C, dark: 0xFF9187)

    /// Sits between the panel's material and its content. `.regularMaterial` is translucent, so
    /// on its own the panel's base color is whatever is behind the window — a dark desktop drags
    /// a light panel down to grey and swallows `inkFaint`. This layer keeps the base predictable
    /// while the material still supplies the blur.
    static let panelBackground = dynamic(light: 0xFCFAF5, dark: 0x201F1C, lightAlpha: 0.88, darkAlpha: 0.88)

    /// A HUD-style toast: dark chip in light mode, light chip in dark mode — always contrasty,
    /// independent of the panel's own background.
    static let toastBackground = dynamic(light: 0x2B2A27, dark: 0xECE6DB)
    static let toastInk = dynamic(light: 0xFCFAF5, dark: 0x201F1C)

    static let panelCorner: CGFloat = 28
    static let formCorner: CGFloat = 20
    static let rowCorner: CGFloat = 18
    static let controlCorner: CGFloat = 14
    static let panelWidth: CGFloat = 396

    /// Four identity colors: mint, apricot, sky, lilac. `AccountHue` picks one per account by
    /// hashing its alias, applied to its avatar and plan chip.
    enum AccountHue: Int, CaseIterable {
        case mint, apricot, sky, lilac

        var tint: Color {
            switch self {
            case .mint: dynamic(light: 0x45B79A, dark: 0x45B79A, lightAlpha: 0.22, darkAlpha: 0.30)
            case .apricot: dynamic(light: 0xF2A54A, dark: 0xF2A54A, lightAlpha: 0.22, darkAlpha: 0.30)
            case .sky: dynamic(light: 0x5B9BD8, dark: 0x5B9BD8, lightAlpha: 0.22, darkAlpha: 0.30)
            case .lilac: dynamic(light: 0x9B7FD1, dark: 0x9B7FD1, lightAlpha: 0.22, darkAlpha: 0.30)
            }
        }

        var ink: Color {
            switch self {
            case .mint: dynamic(light: 0x1F6E5C, dark: 0x9EE3D0)
            case .apricot: dynamic(light: 0x8A5416, dark: 0xFBCB93)
            case .sky: dynamic(light: 0x2C5C86, dark: 0xB9D6F2)
            case .lilac: dynamic(light: 0x5A3F8E, dark: 0xD9C9F5)
            }
        }

        var ring: Color {
            switch self {
            case .mint: dynamic(light: 0x45B79A, dark: 0x45B79A, lightAlpha: 0.55, darkAlpha: 0.6)
            case .apricot: dynamic(light: 0xF2A54A, dark: 0xF2A54A, lightAlpha: 0.55, darkAlpha: 0.6)
            case .sky: dynamic(light: 0x5B9BD8, dark: 0x5B9BD8, lightAlpha: 0.55, darkAlpha: 0.6)
            case .lilac: dynamic(light: 0x9B7FD1, dark: 0x9B7FD1, lightAlpha: 0.55, darkAlpha: 0.6)
            }
        }

        /// FNV-1a over the alias: identity colors stay stable when the list reorders or shrinks
        /// (Swift's `hashValue` is per-process seeded, so it can't be used here).
        static func forAlias(_ alias: String) -> AccountHue {
            var h: UInt64 = 0xcbf2_9ce4_8422_2325
            for byte in alias.utf8 {
                h = (h ^ UInt64(byte)) &* 0x0000_0100_0000_01b3
            }
            return allCases[Int(h % UInt64(allCases.count))]
        }
    }

    /// Severity tiers for a "percent remaining" reading — plenty left, getting low, almost
    /// gone. The single home of the 50/20 thresholds; each surface maps tiers to its own colors.
    enum Severity {
        case plenty, low, critical

        init(remainingPercent: Double) {
            self = if remainingPercent >= 50 {
                .plenty
            } else if remainingPercent >= 20 {
                .low
            } else {
                .critical
            }
        }
    }

    static func severity(remainingPercent: Double) -> Color {
        switch Severity(remainingPercent: remainingPercent) {
        case .plenty: success
        case .low: warning
        case .critical: critical
        }
    }

    private static func dynamic(light: Int, dark: Int, lightAlpha: Double = 1, darkAlpha: Double = 1) -> Color {
        Color(nsColor: NSColor(name: nil) { appearance in
            let isDark = appearance.bestMatch(from: [.aqua, .darkAqua]) == .darkAqua
            let hex = isDark ? dark : light
            let alpha = isDark ? darkAlpha : lightAlpha
            return NSColor(
                srgbRed: CGFloat((hex >> 16) & 0xFF) / 255,
                green: CGFloat((hex >> 8) & 0xFF) / 255,
                blue: CGFloat(hex & 0xFF) / 255,
                alpha: alpha
            )
        })
    }
}
