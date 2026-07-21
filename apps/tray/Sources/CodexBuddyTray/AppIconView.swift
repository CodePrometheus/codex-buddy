import AppKit
import SwiftUI

/// The app-bundle icon (`.icns`), rendered from the same `BuddyWordmark` shapes as the in-app
/// logo so the two can never drift. Colors are fixed (not `Theme`'s dynamic ones): an `.icns`
/// is a baked bitmap and can't follow light/dark. Layout follows Big Sur's icon grid — an
/// 824pt rounded square centered on a 1024pt canvas, leaving the margin the Dock expects.
struct AppIconView: View {
    private static let canvas: CGFloat = 1024
    private static let body: CGFloat = 824

    // Ink bounding box of the wordmark inside `BuddyWordmark`'s 700x260 canvas, computed from the
    // real path data (letters + the `>_` mark's 8pt stroke halo + underscore).
    private static let markCenterX: CGFloat = 217.873
    private static let markCenterY: CGFloat = 140.680
    private static let markWidth: CGFloat = 342.785
    private static let targetWidth: CGFloat = 600 // wordmark width inside the 824 body
    private static let scale = targetWidth / markWidth

    // B2 palette: deep-berry field, warm off-white letters, apricot-berry gradient on `>_`.
    private static let field = Color(red: 62 / 255, green: 33 / 255, blue: 41 / 255)      // #3E2129
    private static let letters = Color(red: 242 / 255, green: 230 / 255, blue: 220 / 255) // #F2E6DC
    private static let markA = Color(red: 226 / 255, green: 155 / 255, blue: 172 / 255)   // #E29BAC
    private static let markB = Color(red: 246 / 255, green: 203 / 255, blue: 166 / 255)   // #F6CBA6

    var body: some View {
        ZStack {
            RoundedRectangle(cornerRadius: Self.body * 0.2237, style: .continuous)
                .fill(Self.field)
                .frame(width: Self.body, height: Self.body)

            BuddyWordmark(letterColor: Self.letters, markStart: Self.markA, markEnd: Self.markB)
                .frame(width: 700, height: 260)
                .scaleEffect(Self.scale)
                // `.scaleEffect` anchors on the 700x260 canvas center (350,130); this offset slides
                // the wordmark's own ink center onto the icon center so it sits truly centered.
                .offset(x: (350 - Self.markCenterX) * Self.scale, y: (130 - Self.markCenterY) * Self.scale)
        }
        .frame(width: Self.canvas, height: Self.canvas)
    }
}

/// Renders `AppIconView` to a 1024x1024 PNG. Invoked by the packaging script via the app's hidden
/// `--render-icon <path>` flag, so the icon comes from the exact same code the UI uses.
@MainActor
func renderAppIconPNG(to path: String) -> Bool {
    let renderer = ImageRenderer(content: AppIconView())
    renderer.scale = 1 // AppIconView is already 1024pt, so 1x → 1024px
    guard let image = renderer.nsImage,
          let tiff = image.tiffRepresentation,
          let rep = NSBitmapImageRep(data: tiff),
          let png = rep.representation(using: .png, properties: [:])
    else { return false }
    do {
        try png.write(to: URL(fileURLWithPath: path))
        return true
    } catch {
        return false
    }
}
