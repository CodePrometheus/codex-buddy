import SwiftUI

/// A circular, per-account identity badge: tinted fill + matching ring, like a sticker.
struct AvatarView: View {
    let initial: String
    let hue: Theme.AccountHue
    var size: CGFloat = 32

    var body: some View {
        Text(initial)
            .font(.system(size: size * 0.42, weight: .semibold, design: .rounded))
            .foregroundStyle(hue.ink)
            .frame(width: size, height: size)
            .background(hue.tint, in: Circle())
            .overlay(Circle().strokeBorder(hue.ring, lineWidth: 1.5))
    }
}
