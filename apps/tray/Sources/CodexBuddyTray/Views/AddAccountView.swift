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
        .background(RoundedRectangle(cornerRadius: Theme.rowCorner, style: .continuous).fill(Color.clear))
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
                .background(Theme.accent, in: RoundedRectangle(cornerRadius: 14))

                Button("Cancel") { isLoading = false }
                    .buttonStyle(.plain)
                    .font(.system(size: 11.5, weight: .semibold))
                    .foregroundStyle(Theme.inkMuted)
                    .frame(maxWidth: .infinity)
            } else {
                TextField("Alias, e.g. freelance", text: $alias)
                    .textFieldStyle(.plain)
                    .font(.system(size: 12.5))
                    .padding(9)
                    .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 14))
                    .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Theme.hairline, lineWidth: 1))
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
                .background(Theme.accent, in: RoundedRectangle(cornerRadius: 14))
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
        .background(Theme.chip, in: RoundedRectangle(cornerRadius: 20, style: .continuous))
    }

    private func startLogin() {
        let trimmed = alias.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        isLoading = true
        Task {
            let ok = await store.add(trimmed)
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

    private func importFile() {
        let panel = NSOpenPanel()
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.title = "Import auth.json"
        guard panel.runModal() == .OK, let url = panel.url else { return }
        let trimmed = alias.trimmingCharacters(in: .whitespaces)
        let suggested = trimmed.isEmpty ? url.deletingPathExtension().lastPathComponent : trimmed
        store.importAuthJSON(path: url.path, alias: suggested)
        expanded = false
        alias = ""
        onToast("Imported \(suggested)")
    }
}
