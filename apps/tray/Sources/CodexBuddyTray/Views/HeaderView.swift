import CodexBuddyFFI
import SwiftUI

struct HeaderView: View {
    let doctorChecks: [DoctorCheck]
    var onOpenDoctorDetail: () -> Void

    var body: some View {
        HStack {
            HStack(spacing: 8) {
                BrandMark().frame(width: 18, height: 18)
                Text("codex-buddy").font(.system(size: 13, weight: .semibold))
            }
            Spacer()
            DoctorPill(checks: doctorChecks, onOpenDetail: onOpenDoctorDetail)
        }
        .padding(.horizontal, 20)
        .padding(.top, 18)
        .padding(.bottom, 12)
    }
}

/// A minimal smiling face, echoing the "buddy" in the name.
private struct BrandMark: View {
    var body: some View {
        ZStack {
            Circle().fill(
                LinearGradient(colors: [Theme.accent, Theme.accent.opacity(0.7)], startPoint: .topLeading, endPoint: .bottomTrailing)
            )
            GeometryReader { geo in
                let s = geo.size.width
                ZStack {
                    Circle().fill(Theme.accentInk).frame(width: s * 0.1, height: s * 0.1)
                        .position(x: s * 0.36, y: s * 0.46)
                    Circle().fill(Theme.accentInk).frame(width: s * 0.1, height: s * 0.1)
                        .position(x: s * 0.64, y: s * 0.46)
                    SmilePath()
                        .stroke(Theme.accentInk, style: StrokeStyle(lineWidth: max(1, s * 0.09), lineCap: .round))
                        .frame(width: s * 0.32, height: s * 0.16)
                        .position(x: s * 0.5, y: s * 0.65)
                }
            }
        }
    }
}

private struct SmilePath: Shape {
    func path(in rect: CGRect) -> Path {
        var p = Path()
        p.move(to: CGPoint(x: 0, y: 0))
        p.addQuadCurve(to: CGPoint(x: rect.width, y: 0), control: CGPoint(x: rect.width / 2, y: rect.height))
        return p
    }
}
