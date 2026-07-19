import SwiftUI

/// The row's "more actions" as a horizontal icon pill instead of a tall text menu — same four
/// actions, a fraction of the height, closer to a Finder/Safari toolbar than a dropdown.
struct RowActionBar: View {
    let isActive: Bool
    var onRename: () -> Void
    var onCopyPath: () -> Void
    var onRunInTerminal: () -> Void
    var onRemove: () -> Void

    var body: some View {
        HStack(spacing: 2) {
            button("pencil", "Rename", action: onRename)
            button("doc.on.doc", "Copy CODEX_HOME", action: onCopyPath)
            button("terminal", "Run in Terminal", action: onRunInTerminal)
            Divider().frame(height: 18).padding(.horizontal, 3)
            button("trash", isActive ? "Switch away from this account first" : "Remove", tint: .critical, disabled: isActive, action: onRemove)
        }
        .padding(6)
    }

    private enum Tint { case normal, critical }

    @ViewBuilder
    private func button(_ systemImage: String, _ tooltip: String, tint: Tint = .normal, disabled: Bool = false, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 13, weight: .medium))
                .frame(width: 32, height: 32)
        }
        .buttonStyle(.plain)
        .foregroundStyle(disabled ? Theme.inkFaint : (tint == .critical ? Theme.critical : Theme.ink))
        .disabled(disabled)
        .help(tooltip)
    }
}
