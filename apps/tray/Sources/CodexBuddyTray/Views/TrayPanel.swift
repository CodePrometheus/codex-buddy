import SwiftUI

struct TrayPanel: View {
    @ObservedObject var store: AccountStore

    @State private var showDoctor = false
    @State private var toast: String?
    @State private var toastTask: Task<Void, Never>?

    var body: some View {
        ZStack {
            mainContent
                .opacity(showDoctor ? 0 : 1)
            if showDoctor {
                DoctorSheet(checks: store.doctorChecks) {
                    withAnimation(.easeOut(duration: 0.2)) { showDoctor = false }
                }
                .transition(.move(edge: .trailing))
            }
        }
        .overlay(alignment: .bottom) { toastView }
        .frame(width: Theme.panelWidth)
        .background(.regularMaterial)
        .clipShape(RoundedRectangle(cornerRadius: Theme.panelCorner, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: Theme.panelCorner, style: .continuous).strokeBorder(Theme.hairline, lineWidth: 1))
        .shadow(color: .black.opacity(0.16), radius: 24, y: 8)
        .background(ClearWindowBackground())
        .onAppear {
            store.refresh()
            store.refreshDoctor()
        }
    }

    private var mainContent: some View {
        VStack(spacing: 0) {
            HeaderView(doctorChecks: store.doctorChecks) {
                withAnimation(.easeOut(duration: 0.2)) { showDoctor = true }
            }

            if let active = store.activeAccount, let index = store.accounts.firstIndex(where: { $0.alias == active.alias }) {
                HeroView(account: active, hue: .forIndex(index))
                Divider().padding(.horizontal, 20)
            }

            ScrollView {
                VStack(spacing: 4) {
                    HStack {
                        Text("ACCOUNTS")
                            .font(.system(size: 10.5, weight: .semibold))
                            .tracking(0.8)
                            .foregroundStyle(Theme.inkFaint)
                        Spacer()
                    }
                    .padding(.horizontal, 10)
                    .padding(.top, 10)
                    .padding(.bottom, 6)

                    ForEach(Array(store.accounts.enumerated()), id: \.element.alias) { index, account in
                        AccountRow(account: account, hue: .forIndex(index), store: store, onToast: showToast)
                    }

                    AddAccountView(store: store, onToast: showToast)
                        .padding(.top, 2)
                }
                .padding(.horizontal, 6)
                .padding(.bottom, 12)
            }
            .frame(minHeight: 220, maxHeight: 320)
        }
    }

    @ViewBuilder
    private var toastView: some View {
        if let toast {
            Text(toast)
                .font(.system(size: 11.5))
                .foregroundStyle(Theme.toastInk)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 13)
                .padding(.vertical, 9)
                .background(Theme.toastBackground, in: RoundedRectangle(cornerRadius: 12))
                .padding(.horizontal, 16)
                .padding(.bottom, showDoctor ? 60 : 16)
                .transition(.move(edge: .bottom).combined(with: .opacity))
        }
    }

    private func showToast(_ message: String) {
        toastTask?.cancel()
        withAnimation(.easeOut(duration: 0.18)) { toast = message }
        toastTask = Task {
            try? await Task.sleep(for: .seconds(3.2))
            guard !Task.isCancelled else { return }
            withAnimation(.easeOut(duration: 0.18)) { toast = nil }
        }
    }
}
