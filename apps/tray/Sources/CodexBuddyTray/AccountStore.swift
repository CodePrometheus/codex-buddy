import CodexBuddyFFI
import SwiftUI

/// Owns all core state for the panel; every mutation re-reads through the FFI so the UI never
/// drifts from what `codex-buddy` on disk actually thinks. `ObservableObject`, not the newer
/// `@Observable` macro, to stay on the app's macOS 13 floor.
@MainActor
final class AccountStore: ObservableObject {
    @Published private(set) var accounts: [Account] = []
    @Published private(set) var doctorChecks: [DoctorCheck] = []
    @Published var lastError: String?

    /// Opt-in: when on, usage is fetched live by driving `codex app-server` per account.
    /// Off by default — the tray stays fully local until the user flips it.
    @Published var liveUsageEnabled: Bool {
        didSet {
            UserDefaults.standard.set(liveUsageEnabled, forKey: Self.liveUsageKey)
            if liveUsageEnabled {
                refreshLiveUsage(force: true)
            } else {
                liveUsage = [:]
                refresh()
            }
        }
    }
    @Published private(set) var liveRefreshInFlight = false

    /// Successful live fetches per alias; local session data stays the fallback for the rest.
    private var liveUsage: [String: [UsageWindow]] = [:]
    private var lastLiveRefresh: Date?
    private static let liveUsageKey = "liveUsageViaCodex"
    /// Panel-open refreshes reuse a recent result; only an explicit "Refresh now" bypasses this.
    private static let liveRefreshInterval: TimeInterval = 300

    init() {
        liveUsageEnabled = UserDefaults.standard.bool(forKey: Self.liveUsageKey)
    }

    var activeAccount: Account? { accounts.first(where: \.isActive) }

    func refresh() {
        run { try listAccounts() } onSuccess: { self.accounts = self.withLiveUsage($0) }
    }

    /// Refetch live usage for every account, one `codex app-server` probe each, in parallel off
    /// the main actor. Merges whatever succeeded; a failure surfaces once without wiping rows.
    func refreshLiveUsage(force: Bool = false) {
        guard liveUsageEnabled, !liveRefreshInFlight else { return }
        if !force, let last = lastLiveRefresh,
           Date().timeIntervalSince(last) < Self.liveRefreshInterval {
            return
        }
        liveRefreshInFlight = true
        let aliases = accounts.map(\.alias)
        Task {
            let (fetched, failure) = await Self.fetchLiveUsage(aliases)
            liveUsage.merge(fetched) { _, new in new }
            lastLiveRefresh = Date()
            liveRefreshInFlight = false
            accounts = withLiveUsage(accounts)
            if let failure { lastError = failure }
        }
    }

    private func withLiveUsage(_ accounts: [Account]) -> [Account] {
        guard liveUsageEnabled else { return accounts }
        return accounts.map { account in
            var merged = account
            if let live = liveUsage[account.alias] { merged.usage = live }
            return merged
        }
    }

    private nonisolated static func fetchLiveUsage(
        _ aliases: [String]
    ) async -> ([String: [UsageWindow]], String?) {
        await withTaskGroup(of: (String, Result<RemoteUsage, Error>).self) { group in
            for alias in aliases {
                group.addTask { (alias, Result { try fetchRemoteUsage(alias: alias) }) }
            }
            var fetched: [String: [UsageWindow]] = [:]
            var failure: String?
            for await (alias, result) in group {
                switch result {
                case .success(let usage): fetched[alias] = usage.windows
                case .failure(let error): failure = "Live usage for \(alias): \(error)"
                }
            }
            return (fetched, failure)
        }
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
            let value = try body()
            // Clear before onSuccess: a nested run (e.g. the refresh after a mutation) may set
            // its own error, which clearing afterwards would silently wipe.
            lastError = nil
            onSuccess(value)
            return true
        } catch {
            lastError = "\(error)"
            return false
        }
    }
}
