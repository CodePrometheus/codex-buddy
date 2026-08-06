import CodexBuddyFFI
import SwiftUI

/// The active account as a soft card washed in its identity hue — identity on the left, usage
/// ring on the right, and the entry into the trends screen tucked under the identity. The card
/// replaces a hard divider: the shape itself separates the hero from the list.
struct HeroView: View {
    let account: Account
    let hue: Theme.AccountHue
    var onOpenTrends: () -> Void = {}

    var body: some View {
        HStack(alignment: .center, spacing: 18) {
            VStack(alignment: .leading, spacing: 11) {
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

                trendsEntry
            }

            Spacer(minLength: 12)

            if !account.usage.isEmpty {
                Button(action: onOpenTrends) {
                    usageStat
                }
                .buttonStyle(.plain)
                .help("Usage trends")
            }
        }
        .padding(16)
        .background(
            hue.tint.opacity(0.45),
            in: RoundedRectangle(cornerRadius: 22, style: .continuous)
        )
        .padding(.horizontal, 14)
        .padding(.bottom, 4)
    }

    private var trendsEntry: some View {
        Button(action: onOpenTrends) {
            HStack(spacing: 5) {
                Image(systemName: "chart.bar.fill").font(.system(size: 9))
                Text("Usage trends").font(.system(size: 10.5, weight: .medium))
                Image(systemName: "chevron.right").font(.system(size: 8, weight: .semibold))
            }
            .foregroundStyle(Theme.inkMuted)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(Theme.chip, in: Capsule())
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
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
