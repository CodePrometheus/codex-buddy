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
                    .padding(10)
            } else {
                normalRow
            }
        }
        .background(
            RoundedRectangle(cornerRadius: Theme.rowCorner, style: .continuous)
                .fill(account.isActive ? Theme.rowActive : (isHovering ? Theme.chip : .clear))
        )
        .overlay(
            RoundedRectangle(cornerRadius: Theme.rowCorner, style: .continuous)
                .strokeBorder(account.isActive ? Theme.rowActiveBorder : .clear, lineWidth: 1)
        )
        .onHover { isHovering = $0 }
    }

    private var normalRow: some View {
        HStack(spacing: 7) {
            switchTarget
            if showActions {
                actionBar
                    .padding(.trailing, 10)
            } else {
                overflowButton
                    .padding(.trailing, 10)
            }
        }
    }

    @ViewBuilder
    private var switchTarget: some View {
        if isRenaming {
            switchTargetContent
        } else {
            Button(action: activateAccount) {
                switchTargetContent
            }
            .buttonStyle(.plain)
            .accessibilityLabel(
                account.isActive ? "\(account.alias), current account" : "Switch to \(account.alias)"
            )
        }
    }

    private var switchTargetContent: some View {
        HStack(spacing: 10) {
            AvatarView(initial: account.initial, hue: hue, size: 32)
            aliasAndEmail
            Spacer(minLength: 6)
            if !showActions {
                accountStatus
            }
        }
        .padding(.leading, 10)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
    }

    private var actionBar: some View {
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
                // No success toast: Terminal taking focus closes the panel before it could show.
                if !TerminalLauncher.run(alias: account.alias) {
                    onToast("Could not prepare the Terminal launch script")
                }
            },
            onRemove: {
                showActions = false
                isConfirmingRemove = true
            },
            onDismiss: { showActions = false }
        )
        .fixedSize()
        .transition(.opacity)
    }

    /// Fixed-size so it's never the thing that gets squeezed — `aliasAndEmail` truncates first.
    private var accountStatus: some View {
        HStack(spacing: 7) {
            usageBadge
            if account.isRunning {
                Circle().fill(Theme.success).frame(width: 7, height: 7)
                    .help("Running via parallel session")
            }
            if account.isActive {
                Image(systemName: "checkmark.circle.fill")
                    .font(.system(size: 14, weight: .semibold))
                    .foregroundStyle(Theme.accent)
            }
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
    }

    @ViewBuilder
    private var usageBadge: some View {
        if let usage = account.tightestUsage {
            HStack(spacing: 3) {
                Text("\(Int(usage.remainingPercent))%")
                    .foregroundStyle(Theme.severity(remainingPercent: usage.remainingPercent))
                Text(usage.label)
                    .foregroundStyle(Theme.inkFaint)
            }
            .font(.system(size: 11, weight: .semibold))
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(
                Theme.severity(remainingPercent: usage.remainingPercent).opacity(0.1),
                in: Capsule()
            )
        }
    }

    private var overflowButton: some View {
        Button {
            withAnimation(.easeOut(duration: 0.15)) { showActions = true }
        } label: {
            Image(systemName: "ellipsis")
                .font(.system(size: 13, weight: .medium))
                .frame(width: 26, height: 26)
                // Without this the hit area is the glyph's own ink — three dots roughly 13x4pt —
                // not the frame, which made the button very hard to land on.
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .foregroundStyle(Theme.inkMuted)
        .opacity(isHovering ? 1 : 0.55)
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

    private func activateAccount() {
        guard !account.isActive else { return }
        if store.switchTo(account.alias) {
            onToast("Switched to \(account.alias)")
        } else if let error = store.lastError {
            onToast(error)
        }
    }

    private func copyHomePath() {
        guard let path = store.homeDirectory(for: account.alias) else {
            onToast("Could not read the account's home directory")
            return
        }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(path, forType: .string)
        onToast("Copied \(path)")
    }
}
