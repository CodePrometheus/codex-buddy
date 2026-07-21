import SwiftUI

/// The row's "more actions" as a horizontal icon strip instead of a tall text menu — same four
/// actions, a fraction of the height, closer to a Finder/Safari toolbar than a dropdown.
///
/// Expands inline within the row rather than in a `.popover`: the strip is ~150pt wide and the
/// trigger sits against the panel's right edge, so a popover centered on that anchor hung well
/// outside the panel and over whatever was behind it.
struct RowActionBar: View {
    let isActive: Bool
    var onRename: () -> Void
    var onCopyPath: () -> Void
    var onRunInTerminal: () -> Void
    var onRemove: () -> Void
    var onDismiss: () -> Void

    var body: some View {
        HStack(spacing: 2) {
            button("pencil", "Rename", action: onRename)
            button("doc.on.doc", "Copy CODEX_HOME", action: onCopyPath)
            button("terminal", "Run in Terminal", action: onRunInTerminal)
            button(
                "trash",
                isActive ? "Switch away from this account first" : "Remove",
                tint: .critical,
                disabled: isActive,
                action: onRemove
            )
            Divider().frame(height: 16).padding(.horizontal, 2)
            button("xmark", "Close", action: onDismiss)
        }
    }

    private enum Tint { case normal, critical }

    @ViewBuilder
    private func button(_ systemImage: String, _ tooltip: String, tint: Tint = .normal, disabled: Bool = false, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Image(systemName: systemImage)
                .font(.system(size: 12.5, weight: .medium))
                .frame(width: 26, height: 26)
                // `.plain` hit-tests the glyph's own ink, not the frame — without this these are
                // as hard to land on as the ellipsis trigger used to be.
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        // `inkFaint` is for text that should recede; on a disabled *button* it dropped below the
        // contrast floor and the icon read as an empty gap. `inkMuted` still says "unavailable"
        // while staying recognizable.
        .foregroundStyle(disabled ? Theme.inkMuted : (tint == .critical ? Theme.critical : Theme.ink))
        .disabled(disabled)
        .help(tooltip)
    }
}
