import SwiftUI

/// Habit-tracker style daily quota pills: one per day with its weekday initial beneath, today
/// rightmost and highlighted. Each fill's height is that day's lowest remaining percent,
/// severity-colored; days without samples keep only the faint track, so the fixed grid reads
/// as a designed fixture from the very first sample. Chrome-free — callers compose the card.
struct TrendBars: View {
    /// (ts, remainingPercent) samples, any order.
    let points: [(ts: Int64, remaining: Double)]
    var barWidth: CGFloat = 8
    var barHeight: CGFloat = 24
    var spacing: CGFloat = 7
    var letterSize: CGFloat = 8

    private struct Day {
        let initial: String
        let remaining: Double?
        let isToday: Bool
    }

    /// Oldest first, 7 entries ending today; each carries the day's tightest reading.
    private var days: [Day] {
        let calendar = Calendar.current
        let today = calendar.startOfDay(for: Date())
        let symbols = calendar.veryShortWeekdaySymbols
        return (0..<7).reversed().compactMap { offset in
            guard let start = calendar.date(byAdding: .day, value: -offset, to: today),
                let end = calendar.date(byAdding: .day, value: 1, to: start)
            else { return nil }
            let range = Int64(start.timeIntervalSince1970)..<Int64(end.timeIntervalSince1970)
            return Day(
                initial: symbols[calendar.component(.weekday, from: start) - 1],
                remaining: points.filter { range.contains($0.ts) }.map(\.remaining).min(),
                isToday: offset == 0
            )
        }
    }

    var body: some View {
        HStack(alignment: .bottom, spacing: spacing) {
            ForEach(Array(days.enumerated()), id: \.offset) { _, day in
                VStack(spacing: 4) {
                    ZStack(alignment: .bottom) {
                        Capsule()
                            .fill(Theme.chip)
                            .frame(width: barWidth, height: barHeight)
                        if let remaining = day.remaining {
                            let clamped = min(max(remaining, 0), 100)
                            Capsule()
                                .fill(Theme.severity(remainingPercent: clamped))
                                .frame(width: barWidth, height: max(5, barHeight * clamped / 100))
                        }
                    }
                    Text(day.initial)
                        .font(.system(size: letterSize, weight: day.isToday ? .bold : .medium))
                        .foregroundStyle(day.isToday ? Theme.accent : Theme.inkFaint)
                }
            }
        }
    }
}
