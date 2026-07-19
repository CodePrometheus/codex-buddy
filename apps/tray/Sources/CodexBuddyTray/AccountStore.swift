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

    @discardableResult
    func switchTo(_ alias: String) -> Bool {
        run { try switchAccount(alias: alias) } onSuccess: { self.refresh() }
    }

    @discardableResult
    func remove(_ alias: String) -> Bool {
        run { try removeAccount(alias: alias) } onSuccess: { self.refresh() }
    }

    @discardableResult
    func rename(_ old: String, to new: String) -> Bool {
        run { try renameAccount(oldAlias: old, newAlias: new) } onSuccess: { self.refresh() }
    }

    func homeDirectory(for alias: String) -> String? {
        try? accountHome(alias: alias)
    }

    private var addInFlight = false

    /// Runs `codex login` (blocking, opens the system browser). Callers must show a loading
    /// state for the duration; the FFI call itself is dispatched off the main actor.
    ///
    /// `codex login` can't actually be interrupted once started — there's no cancellation hook
    /// through the blocking FFI call — so a caller-side "Cancel" can only stop the *panel* from
    /// waiting on it. `addInFlight` still refuses a second concurrent attempt while the first is
    /// genuinely running, which is what actually matters: it stops two `codex login` processes
    /// racing for the same or a different alias.
    func add(_ alias: String) async -> Bool {
        guard !addInFlight else {
            lastError = "A login is already in progress"
            return false
        }
        addInFlight = true
        defer { addInFlight = false }

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

    @discardableResult
    func importAuthJSON(path: String, alias: String) -> Bool {
        run { try CodexBuddyFFI.importAccount(authJsonPath: path, alias: alias) } onSuccess: { self.refresh() }
    }

    @discardableResult
    private func run<T>(_ body: () throws -> T, onSuccess: (T) -> Void) -> Bool {
        do {
            onSuccess(try body())
            lastError = nil
            return true
        } catch {
            lastError = "\(error)"
            return false
        }
    }
}
