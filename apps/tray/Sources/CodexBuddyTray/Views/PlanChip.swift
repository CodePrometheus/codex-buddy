import SwiftUI

/// The account's plan as a soft, hue-tinted capsule — shared between the hero and the rows so
/// the same account always wears the same chip.
struct PlanChip: View {
    let text: String
    let hue: Theme.AccountHue

    var body: some View {
        Text(text.uppercased())
            .font(.system(size: 9.5, weight: .semibold))
            .tracking(0.4)
            .foregroundStyle(hue.ink)
            .padding(.horizontal, 7)
            .padding(.vertical, 2)
            .background(hue.tint, in: Capsule())
    }
}
