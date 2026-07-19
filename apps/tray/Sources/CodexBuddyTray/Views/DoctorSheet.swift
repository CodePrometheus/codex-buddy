import AppKit
import CodexBuddyFFI
import SwiftUI

struct DoctorSheet: View {
    let checks: [DoctorCheck]
    var onBack: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 8) {
                Button(action: onBack) {
                    HStack(spacing: 3) {
                        Image(systemName: "chevron.left")
                        Text("Back")
                    }
                }
                .buttonStyle(.plain)
                .font(.system(size: 12.5, weight: .semibold))
                .foregroundStyle(Theme.accent)
                Spacer()
                Text("Doctor").font(.system(size: 13, weight: .semibold))
                Spacer()
                Color.clear.frame(width: 40)
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 12)
            .overlay(alignment: .bottom) { Divider() }

            ScrollView {
                VStack(spacing: 2) {
                    ForEach(Array(checks.enumerated()), id: \.offset) { _, check in
                        HStack(alignment: .top, spacing: 10) {
                            Image(systemName: icon(for: check.level))
                                .foregroundStyle(color(for: check.level))
                                .frame(width: 16)
                                .padding(.top, 1)
                            Text(check.message)
                                .font(.system(size: 12.5))
                                .foregroundStyle(Theme.ink)
                                .fixedSize(horizontal: false, vertical: true)
                            Spacer(minLength: 0)
                        }
                        .padding(10)
                    }
                }
                .padding(8)
            }

            Divider()
            Button {
                let report = checks.map { "[\($0.level)] \($0.message)" }.joined(separator: "\n")
                NSPasteboard.general.clearContents()
                NSPasteboard.general.setString(report, forType: .string)
            } label: {
                Label("Copy Report", systemImage: "doc.on.doc")
                    .font(.system(size: 12, weight: .semibold))
                    .frame(maxWidth: .infinity)
            }
            .buttonStyle(.plain)
            .padding(9)
            .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Theme.hairline, lineWidth: 1))
            .padding(10)
        }
    }

    private func icon(for level: CheckLevel) -> String {
        switch level {
        case .pass: "checkmark.circle.fill"
        case .warn: "exclamationmark.triangle.fill"
        case .fail: "xmark.circle.fill"
        }
    }

    private func color(for level: CheckLevel) -> Color {
        switch level {
        case .pass: Theme.success
        case .warn: Theme.warning
        case .fail: Theme.critical
        }
    }
}
