import CodexBuddyFFI
import SwiftUI

/// Concentric rings — one per window actually present, shortest window outermost — filled by
/// percent remaining and colored by how tight each is. Absent windows render nothing: an empty
/// placeholder ring would read as a full (or dead) window that doesn't exist. The tightest
/// window's number sits centered and large — the ring's negative space is what gives it room to
/// read as *the* number, not just another line of text.
struct UsageRingView: View {
    let windows: [UsageWindow]
    var diameter: CGFloat = 56
    var showsCenterLabel: Bool = false

    var body: some View {
        let shown = windows.sorted { $0.windowMinutes < $1.windowMinutes }.prefix(2)
        ZStack {
            ForEach(Array(shown.enumerated()), id: \.element.windowMinutes) { index, window in
                ring(window, lineWidth: 5, inset: CGFloat(index) * 8)
            }
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
    private func ring(_ window: UsageWindow, lineWidth: CGFloat, inset: CGFloat) -> some View {
        let remaining = window.remainingPercent / 100
        ZStack {
            Circle().stroke(Theme.chipStrong, lineWidth: lineWidth)
            Circle()
                .trim(from: 0, to: remaining)
                .stroke(
                    Theme.severity(remainingPercent: window.remainingPercent),
                    style: StrokeStyle(lineWidth: lineWidth, lineCap: .round)
                )
                .rotationEffect(.degrees(-90))
                .animation(.easeOut(duration: 0.5), value: remaining)
        }
        .padding(inset)
    }
}
