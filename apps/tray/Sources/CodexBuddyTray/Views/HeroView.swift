import CodexBuddyFFI
import SwiftUI

/// The active account: identity and dual usage ring.
struct HeroView: View {
    let account: Account
    let hue: Theme.AccountHue

    var body: some View {
        HStack(alignment: .top, spacing: 18) {
            VStack(alignment: .leading, spacing: 10) {
                HStack(spacing: 11) {
                    AvatarView(initial: account.initial, hue: hue, size: 40)
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(spacing: 8) {
                            Text(account.alias).font(.system(size: 16, weight: .semibold)).lineLimit(1)
                            if let plan = account.plan {
                                PlanChip(text: plan, hue: hue).fixedSize()
                            }
                        }
                        if let email = account.email {
                            Text(email).font(.system(size: 11.5)).foregroundStyle(Theme.inkMuted).lineLimit(1)
                        }
                    }
                }

            }

            Spacer(minLength: 12)

            if !account.usage.isEmpty {
                usageStat
            }
        }
        .padding(.horizontal, 20)
        .padding(.top, 8)
        .padding(.bottom, 16)
    }

    /// The ring carries the number: negative space around it is what makes the percentage read
    /// as a considered stat rather than another line of text competing with everything else.
    private var usageStat: some View {
        VStack(spacing: 7) {
            UsageRingView(windows: account.usage, diameter: 68, showsCenterLabel: true)
            if let other = account.usage.secondary {
                HStack(spacing: 5) {
                    Circle().fill(Theme.severity(remainingPercent: other.remainingPercent)).frame(width: 5, height: 5)
                    Text("\(other.label) \(Int(other.remainingPercent))%")
                }
                .font(.system(size: 10.5, weight: .medium))
                .foregroundStyle(Theme.inkMuted)
            }
        }
    }
}
