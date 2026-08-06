import CodexBuddyFFI
import SwiftUI

/// Second screen: every account's past week as a habit-tracker card — identity on top, daily
/// quota pills below, current headroom on the right.
struct TrendSheet: View {
    let accounts: [Account]
    let history: [String: [UsagePoint]]
    var onBack: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Button(action: onBack) {
                    HStack(spacing: 3) {
                        Image(systemName: "chevron.left")
                        Text("Back")
                    }
                }
                .buttonStyle(.plain)
                .font(.system(size: 12.5, weight: .semibold))
                .foregroundStyle(Theme.accent)
                Spacer()
                Text("Usage Trends").font(.system(size: 13, weight: .semibold))
                Spacer()
                // Width-only balance for the Back button; the height must be pinned too —
                // Color is greedy on both axes and would stretch the whole header bar.
                Color.clear.frame(width: 40, height: 1)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .overlay(alignment: .bottom) { Divider() }

            ScrollView {
                VStack(spacing: 10) {
                    ForEach(accounts, id: \.alias) { account in
                        card(for: account)
                    }
                }
                .padding(14)
            }
        }
    }

    private func card(for account: Account) -> some View {
        let hue = Theme.AccountHue.forAlias(account.alias)
        let points = trendPoints(for: account)
        return VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 9) {
                AvatarView(initial: account.initial, hue: hue, size: 26)
                Text(account.alias).font(.system(size: 13, weight: .semibold)).lineLimit(1)
                if let plan = account.plan {
                    PlanChip(text: plan, hue: hue).fixedSize()
                }
                Spacer()
                if let tightest = account.usage.tightest {
                    Text("\(Int(tightest.remainingPercent))% left")
                        .font(.system(size: 11, weight: .semibold))
                        .foregroundStyle(
                            Theme.severity(remainingPercent: tightest.remainingPercent))
                }
            }

            if points.isEmpty {
                Text("No usage recorded yet — the week fills in as this account gets used.")
                    .font(.system(size: 11))
                    .foregroundStyle(Theme.inkFaint)
            } else {
                HStack {
                    Spacer(minLength: 0)
                    TrendBars(points: points, barWidth: 16, barHeight: 46, spacing: 14, letterSize: 9)
                    Spacer(minLength: 0)
                }
            }
        }
        .padding(14)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Theme.chip, in: RoundedRectangle(cornerRadius: Theme.formCorner, style: .continuous))
    }

    /// Remaining percent per sample for the account's widest window on record.
    private func trendPoints(for account: Account) -> [(ts: Int64, remaining: Double)] {
        let samples = history[account.alias] ?? []
        guard
            let minutes = account.usage.map(\.windowMinutes).max()
                ?? samples.map(\.windowMinutes).max()
        else { return [] }
        return
            samples
            .filter { $0.windowMinutes == minutes }
            .map { (ts: $0.ts, remaining: max(0, 100 - $0.usedPercent)) }
    }
}
