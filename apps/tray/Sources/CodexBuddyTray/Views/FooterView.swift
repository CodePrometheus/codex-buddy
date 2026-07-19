import SwiftUI

/// Quit lives in the status item's right-click menu now, so this just grounds the panel with a
/// version readout rather than repeating a second "Quit" control.
struct FooterView: View {
    var body: some View {
        HStack {
            Text(versionString)
                .font(.system(size: 10.5, design: .monospaced))
                .foregroundStyle(Theme.inkFaint)
            Spacer()
        }
        .padding(.horizontal, 20)
        .padding(.vertical, 12)
        .overlay(alignment: .top) { Divider() }
    }

    private var versionString: String {
        "v" + (Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.1.0")
    }
}
