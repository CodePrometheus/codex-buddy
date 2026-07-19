import CodexBuddyFFI
import SwiftUI

/// Owns all core state for the panel; every mutation re-reads through the FFI so the UI never
/// drifts from what `codex-buddy` on disk actually thinks. `ObservableObject`, not the newer
/// `@Observable` macro, to stay on the macOS 13 floor `MenuBarExtra(.window)` targets.
@MainActor
final class AccountStore: ObservableObject {
    @Published private(set) var accounts: [Account] = []
    @Published private(set) var doctorChecks: [DoctorCheck] = []
    @Published var lastError: String?

    var activeAccount: Account? { accounts.first(where: \.isActive) }

    func refresh() {
        run { try listAccounts() } onSuccess: { self.accounts = $0 }
    }

    func refreshDoctor() {
        run { try doctor() } onSuccess: { self.doctorChecks = $0 }
    }

    func switchTo(_ alias: String) {
        run { try switchAccount(alias: alias) } onSuccess: { self.refresh() }
    }

    func remove(_ alias: String) {
        run { try removeAccount(alias: alias) } onSuccess: { self.refresh() }
    }

    func rename(_ old: String, to new: String) {
        run { try renameAccount(oldAlias: old, newAlias: new) } onSuccess: { self.refresh() }
    }

    func homeDirectory(for alias: String) -> String? {
        try? accountHome(alias: alias)
    }

    /// Runs `codex login` (blocking, opens the system browser). Callers must show a loading
    /// state for the duration; the FFI call itself is dispatched off the main actor.
    func add(_ alias: String) async -> Bool {
        let failure = await Task.detached(priority: .userInitiated) { () -> String? in
            do {
                try addAccount(alias: alias)
                return nil
            } catch {
                return "\(error)"
            }
        }.value
        guard let failure else {
            refresh()
            return true
        }
        lastError = failure
        return false
    }

    func importAuthJSON(path: String, alias: String) {
        run { try CodexBuddyFFI.importAccount(authJsonPath: path, alias: alias) } onSuccess: { self.refresh() }
    }

    private func run<T>(_ body: () throws -> T, onSuccess: (T) -> Void) {
        do {
            onSuccess(try body())
            lastError = nil
        } catch {
            lastError = "\(error)"
        }
    }
}
