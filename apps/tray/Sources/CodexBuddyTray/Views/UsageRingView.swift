import CodexBuddyFFI
import SwiftUI

/// Concentric 5h/7d rings, filled by percent remaining and colored by how tight each is. The
/// tightest window's number sits centered and large — the ring's negative space is what gives
/// it room to read as *the* number, not just another line of text.
struct UsageRingView: View {
    let windows: [UsageWindow]
    var diameter: CGFloat = 56
    var showsCenterLabel: Bool = false

    var body: some View {
        ZStack {
            ring(windows.fiveHour, lineWidth: 5, inset: 0)
            ring(windows.weekly, lineWidth: 5, inset: 8)
            if showsCenterLabel, let tightest = windows.tightest {
                VStack(spacing: 0) {
                    Text("\(Int(tightest.remainingPercent))")
                        .font(.system(size: diameter * 0.26, weight: .bold, design: .rounded))
                    Text(tightest.label)
                        .font(.system(size: diameter * 0.12, weight: .semibold))
                        .foregroundStyle(Theme.inkFaint)
                }
                .foregroundStyle(Theme.ink)
            }
        }
        .frame(width: diameter, height: diameter)
    }

    @ViewBuilder
    private func ring(_ window: UsageWindow?, lineWidth: CGFloat, inset: CGFloat) -> some View {
        let remaining = (window?.remainingPercent ?? 100) / 100
        ZStack {
            Circle().stroke(Theme.chipStrong, lineWidth: lineWidth)
            Circle()
                .trim(from: 0, to: remaining)
                .stroke(
                    window.map { Theme.severity(remainingPercent: $0.remainingPercent) } ?? Theme.chipStrong,
                    style: StrokeStyle(lineWidth: lineWidth, lineCap: .round)
                )
                .rotationEffect(.degrees(-90))
                .animation(.easeOut(duration: 0.5), value: remaining)
        }
        .padding(inset)
    }
}
