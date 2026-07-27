import CodexBuddyFFI

extension UsageWindow {
    /// The 5h/weekly boundary in minutes, mirroring core's `FIVE_HOUR_MINUTES`.
    static let fiveHourMinutes: Int64 = 300

    /// Named labels for the known windows, generic day/hour/minute fallbacks otherwise — the
    /// window set is whatever codex reports, and it has changed upstream before.
    var label: String {
        switch windowMinutes {
        case Self.fiveHourMinutes: "5h"
        case 10080: "7d"
        case let m where m % 1440 == 0: "\(m / 1440)d"
        case let m where m % 60 == 0: "\(m / 60)h"
        default: "\(windowMinutes)m"
        }
    }

    /// codex reports how much of the window is *used*; the tray shows what's left, matching
    /// how the mockup — and the plain-language "46% left" framing — reads at a glance.
    var remainingPercent: Double { max(0, 100 - usedPercent) }
}

extension [UsageWindow] {
    /// The window that will bite first.
    var tightest: UsageWindow? { self.max(by: { $0.usedPercent < $1.usedPercent }) }

    var fiveHour: UsageWindow? { first { $0.windowMinutes <= UsageWindow.fiveHourMinutes } }

    var weekly: UsageWindow? { first { $0.windowMinutes > UsageWindow.fiveHourMinutes } }

    /// The window that is not the tightest, for a secondary readout.
    var secondary: UsageWindow? {
        guard let tightest else { return nil }
        return first { $0.windowMinutes != tightest.windowMinutes }
    }
}

extension Account {
    /// The tighter of the two usage windows — the one that will bite first.
    var tightestUsage: UsageWindow? { usage.tightest }

    var initial: String {
        String(alias.prefix(1)).uppercased()
    }
}
