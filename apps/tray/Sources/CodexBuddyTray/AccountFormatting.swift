import CodexBuddyFFI

extension UsageWindow {
    var label: String { windowMinutes <= 300 ? "5h" : "7d" }

    /// codex reports how much of the window is *used*; the tray shows what's left, matching
    /// how the mockup — and the plain-language "46% left" framing — reads at a glance.
    var remainingPercent: Double { max(0, 100 - usedPercent) }
}

extension Account {
    /// The tighter of the two usage windows — the one that will bite first — with its label.
    var tightestUsage: UsageWindow? {
        usage.max(by: { $0.usedPercent < $1.usedPercent })
    }

    var initial: String {
        String(alias.prefix(1)).uppercased()
    }
}
