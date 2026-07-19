import CodexBuddyFFI
import SwiftUI

/// "All good" only ever nudges — no issue means nothing to drill into. A real issue opens the
/// detail sheet instead.
struct DoctorPill: View {
    let checks: [DoctorCheck]
    var onOpenDetail: () -> Void

    @State private var nudge = false

    private var hasIssue: Bool { checks.contains { $0.level != .pass } }
    private var tint: Color { hasIssue ? (checks.contains { $0.level == .fail } ? Theme.critical : Theme.warning) : Theme.success }
    private var label: String {
        let issues = checks.filter { $0.level != .pass }.count
        return hasIssue ? "\(issues) issue\(issues == 1 ? "" : "s")" : "All good"
    }

    var body: some View {
        Button {
            hasIssue ? onOpenDetail() : nudgeTap()
        } label: {
            HStack(spacing: 6) {
                Circle().fill(tint).frame(width: 6, height: 6)
                Text(label)
            }
            .font(.system(size: 11, weight: .medium))
            .foregroundStyle(Theme.ink)
            .padding(.horizontal, 9)
            .padding(.vertical, 4)
            .background(Theme.chip, in: Capsule())
        }
        .buttonStyle(.plain)
        .scaleEffect(nudge ? 0.9 : 1)
    }

    /// No issue → nothing to drill into; a quick compress-and-spring-back is enough
    /// acknowledgement that the tap registered.
    private func nudgeTap() {
        withAnimation(.easeOut(duration: 0.1)) { nudge = true }
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.1) {
            withAnimation(.interpolatingSpring(stiffness: 300, damping: 10)) { nudge = false }
        }
    }
}
