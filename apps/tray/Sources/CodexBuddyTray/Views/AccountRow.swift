import AppKit
import CodexBuddyFFI
import SwiftUI

struct AccountRow: View {
    let account: Account
    let hue: Theme.AccountHue
    @ObservedObject var store: AccountStore
    var onToast: (String) -> Void

    @State private var isHovering = false
    @State private var showActions = false
    @State private var isRenaming = false
    @State private var renameText = ""
    @State private var isConfirmingRemove = false
    @FocusState private var renameFocused: Bool

    var body: some View {
        Group {
            if isConfirmingRemove {
                confirmRemoveRow
            } else {
                normalRow
            }
        }
        .padding(10)
        .background(
            RoundedRectangle(cornerRadius: Theme.rowCorner, style: .continuous)
                .fill(account.isActive ? Theme.accentSoft : (isHovering ? Theme.chip : .clear))
        )
        .onHover { isHovering = $0 }
    }

    private var normalRow: some View {
        HStack(spacing: 10) {
            AvatarView(initial: account.initial, hue: hue, size: 32)
            aliasAndEmail
            Spacer(minLength: 6)
            trailingCluster
        }
    }

    /// Fixed-size so it's never the thing that gets squeezed — `aliasAndEmail` truncates first.
    private var trailingCluster: some View {
        HStack(spacing: 7) {
            usageBadge
            if account.isRunning {
                Circle().fill(Theme.success).frame(width: 7, height: 7)
                    .help("Running via parallel session")
            }
            if account.isActive {
                Image(systemName: "checkmark")
                    .font(.system(size: 12, weight: .bold))
                    .foregroundStyle(Theme.accent)
            }
            overflowButton
        }
        .fixedSize()
    }

    @ViewBuilder
    private var aliasAndEmail: some View {
        VStack(alignment: .leading, spacing: 2) {
            if isRenaming {
                TextField("Alias", text: $renameText)
                    .textFieldStyle(.plain)
                    .font(.system(size: 13.5, weight: .semibold))
                    .focused($renameFocused)
                    .onSubmit(commitRename)
                    .onExitCommand { isRenaming = false }
                    .onAppear { renameFocused = true }
            } else {
                HStack(spacing: 7) {
                    Text(account.alias).font(.system(size: 13.5, weight: .semibold)).lineLimit(1)
                    if let plan = account.plan {
                        PlanChip(text: plan, hue: hue).fixedSize()
                    }
                }
                if let email = account.email {
                    Text(email)
                        .font(.system(size: 11.5))
                        .foregroundStyle(Theme.inkMuted)
                        .lineLimit(1)
                }
            }
        }
        .contentShape(Rectangle())
        .onTapGesture {
            guard !isRenaming else { return }
            if !store.switchTo(account.alias), let error = store.lastError {
                onToast(error)
            }
        }
    }

    @ViewBuilder
    private var usageBadge: some View {
        if let usage = account.tightestUsage {
            HStack(spacing: 2) {
                Text("\(Int(usage.remainingPercent))%")
                    .foregroundStyle(Theme.severity(remainingPercent: usage.remainingPercent))
                Text("·\(usage.label)")
                    .foregroundStyle(Theme.inkFaint)
            }
            .font(.system(size: 11.5, weight: .semibold))
        }
    }

    private var overflowButton: some View {
        Button {
            showActions = true
        } label: {
            Image(systemName: "ellipsis")
                .font(.system(size: 13, weight: .medium))
                .frame(width: 22, height: 22)
        }
        .buttonStyle(.plain)
        .foregroundStyle(Theme.inkMuted)
        .opacity(isHovering || showActions ? 1 : 0.55)
        .popover(isPresented: $showActions, arrowEdge: .bottom) {
            RowActionBar(
                isActive: account.isActive,
                onRename: {
                    showActions = false
                    renameText = account.alias
                    isRenaming = true
                },
                onCopyPath: {
                    showActions = false
                    copyHomePath()
                },
                onRunInTerminal: {
                    showActions = false
                    TerminalLauncher.run(alias: account.alias)
                    onToast("Opened Terminal running codex-buddy run \(account.alias)")
                },
                onRemove: {
                    showActions = false
                    isConfirmingRemove = true
                }
            )
        }
    }

    private var confirmRemoveRow: some View {
        HStack(spacing: 12) {
            AvatarView(initial: account.initial, hue: hue, size: 32)
            VStack(alignment: .leading, spacing: 4) {
                Text("Remove \(account.alias)? Your ChatGPT login stays intact — this only forgets it here.")
                    .font(.system(size: 11.5))
                    .foregroundStyle(Theme.inkMuted)
                    .fixedSize(horizontal: false, vertical: true)
                HStack(spacing: 14) {
                    Button("Cancel") { isConfirmingRemove = false }
                        .foregroundStyle(Theme.inkMuted)
                    Button("Remove") {
                        isConfirmingRemove = false
                        let alias = account.alias
                        if store.remove(alias) {
                            onToast("Removed \(alias). You can add it again anytime.")
                        } else if let error = store.lastError {
                            onToast(error)
                        }
                    }
                    .foregroundStyle(Theme.critical)
                }
                .font(.system(size: 12, weight: .semibold))
                .buttonStyle(.plain)
            }
        }
    }

    private func commitRename() {
        let trimmed = renameText.trimmingCharacters(in: .whitespacesAndNewlines)
        isRenaming = false
        guard !trimmed.isEmpty, trimmed != account.alias else { return }
        if store.rename(account.alias, to: trimmed) {
            onToast("Renamed to \(trimmed)")
        } else if let error = store.lastError {
            onToast(error)
        }
    }

    private func copyHomePath() {
        guard let path = store.homeDirectory(for: account.alias) else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(path, forType: .string)
        onToast("Copied \(path)")
    }
}

private struct PlanChip: View {
    let text: String
    let hue: Theme.AccountHue

    var body: some View {
        Text(text.uppercased())
            .font(.system(size: 9.5, weight: .semibold))
            .tracking(0.4)
            .foregroundStyle(hue.ink)
            .padding(.horizontal, 7)
            .padding(.vertical, 2)
            .background(hue.tint, in: Capsule())
    }
}
