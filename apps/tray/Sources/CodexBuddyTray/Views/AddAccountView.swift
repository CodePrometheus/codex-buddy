import AppKit
import SwiftUI

/// Expands in place — no separate window — into an alias field plus login/import, mirroring
/// how `codex-buddy add`/`import` work on the CLI.
struct AddAccountView: View {
    @ObservedObject var store: AccountStore
    var onToast: (String) -> Void

    @State private var expanded = false
    @State private var alias = ""
    @State private var isLoading = false
    @State private var loginTask: Task<Void, Never>?
    @FocusState private var aliasFocused: Bool

    var body: some View {
        if expanded {
            form
        } else {
            idleRow
        }
    }

    private var idleRow: some View {
        Button {
            expanded = true
        } label: {
            HStack(spacing: 12) {
                ZStack {
                    Circle().fill(Theme.chipStrong)
                    Image(systemName: "plus").font(.system(size: 12, weight: .semibold))
                }
                .frame(width: 32, height: 32)
                Text("Add Account").font(.system(size: 12.5, weight: .medium))
                Spacer()
            }
        }
        .buttonStyle(.plain)
        .foregroundStyle(Theme.inkMuted)
        .padding(10)
        .contentShape(RoundedRectangle(cornerRadius: Theme.rowCorner))
    }

    private var form: some View {
        VStack(spacing: 10) {
            HStack {
                Text("New account").font(.system(size: 11.5, weight: .semibold)).foregroundStyle(Theme.inkMuted)
                Spacer()
                Button {
                    expanded = false
                    alias = ""
                } label: {
                    Image(systemName: "xmark").font(.system(size: 12, weight: .semibold))
                }
                .buttonStyle(.plain)
                .foregroundStyle(Theme.inkMuted)
            }

            if isLoading {
                HStack(spacing: 7) {
                    ProgressView().controlSize(.small)
                    Text("Waiting for browser sign-in…")
                }
                .font(.system(size: 12.5, weight: .semibold))
                .foregroundStyle(Theme.accentInk)
                .frame(maxWidth: .infinity)
                .padding(10)
                .background(Theme.accent, in: RoundedRectangle(cornerRadius: Theme.controlCorner))

                Button("Cancel", action: cancelLogin)
                    .buttonStyle(.plain)
                    .font(.system(size: 11.5, weight: .semibold))
                    .foregroundStyle(Theme.inkMuted)
                    .frame(maxWidth: .infinity)
            } else {
                TextField("Alias, e.g. freelance", text: $alias)
                    .textFieldStyle(.plain)
                    .font(.system(size: 12.5))
                    .padding(9)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: Theme.controlCorner))
                    .overlay(RoundedRectangle(cornerRadius: Theme.controlCorner).strokeBorder(Theme.hairline, lineWidth: 1))
                    .focused($aliasFocused)
                    .onAppear { aliasFocused = true }
                    .onSubmit(startLogin)

                Button(action: startLogin) {
                    Text("Continue with ChatGPT Login")
                        .font(.system(size: 12.5, weight: .semibold))
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.plain)
                .foregroundStyle(Theme.accentInk)
                .padding(10)
                .background(Theme.accent, in: RoundedRectangle(cornerRadius: Theme.controlCorner))
                .disabled(alias.trimmingCharacters(in: .whitespaces).isEmpty)

                Button("Import an existing auth.json") { importFile() }
                    .buttonStyle(.plain)
                    .font(.system(size: 11.5))
                    .foregroundStyle(Theme.inkMuted)
                    .underline()
                    .frame(maxWidth: .infinity)
            }
        }
        .padding(14)
        .background(Theme.chip, in: RoundedRectangle(cornerRadius: Theme.formCorner, style: .continuous))
    }

    private func startLogin() {
        let trimmed = alias.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        isLoading = true
        loginTask = Task {
            let ok = await store.add(trimmed)
            // Cancel already dismissed this form; don't reach back into it after the fact —
            // `codex login` itself can't be interrupted, so the account may still land in the
            // registry, but the next panel refresh picks that up on its own.
            guard !Task.isCancelled else { return }
            isLoading = false
            if ok {
                expanded = false
                alias = ""
                onToast("Added \(trimmed)")
            } else if let error = store.lastError {
                onToast(error)
            }
        }
    }

    /// Stops the panel from waiting on this attempt. `codex login` keeps running underneath —
    /// see the note on `AccountStore.add` — but `addInFlight` there still blocks a second
    /// concurrent login, so trying again right away reports "already in progress" instead of
    /// racing a second `codex login` process.
    private func cancelLogin() {
        loginTask?.cancel()
        loginTask = nil
        isLoading = false
    }

    private func importFile() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.title = "Import auth.json"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        let trimmed = alias.trimmingCharacters(in: .whitespaces)
        let suggested = trimmed.isEmpty ? url.deletingPathExtension().lastPathComponent : trimmed
        if store.importAuthJSON(path: url.path, alias: suggested) {
            expanded = false
            alias = ""
            onToast("Imported \(suggested)")
        } else if let error = store.lastError {
            onToast(error)
        }
    }
}
