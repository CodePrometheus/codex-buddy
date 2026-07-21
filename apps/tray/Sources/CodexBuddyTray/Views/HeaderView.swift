import CodexBuddyFFI
import SwiftUI

struct HeaderView: View {
    let doctorChecks: [DoctorCheck]
    @ObservedObject var appearance: AppearanceController
    var onOpenDoctorDetail: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            BuddyWordmark()
                .scaleEffect(wordmarkScale)
                .frame(width: 700 * wordmarkScale, height: 260 * wordmarkScale)
            Spacer()
            DoctorPill(checks: doctorChecks, onOpenDetail: onOpenDoctorDetail)
            appearanceToggle
        }
        .padding(.horizontal, 20)
        .padding(.top, 18)
        .padding(.bottom, 12)
    }

    /// A menu rather than a cycling button: the three modes all carry meaning when the system is
    /// set to "auto", and a cycling icon shows neither what the options are nor where the next
    /// click lands.
    private var appearanceToggle: some View {
        Menu {
            Picker("Appearance", selection: $appearance.mode) {
                ForEach(AppearanceMode.allCases, id: \.self) { mode in
                    Label(mode.title, systemImage: mode.symbolName).tag(mode)
                }
            }
            .pickerStyle(.inline)
            .labelsHidden()
        } label: {
            Image(systemName: appearance.mode.symbolName)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(Theme.inkMuted)
                .frame(width: 26, height: 26)
                .background(Circle().fill(Theme.chip))
                .contentShape(Circle())
        }
        .menuStyle(.borderlessButton)
        .menuIndicator(.hidden)
        .frame(width: 26, height: 26)
        .help("Appearance")
    }

    private let wordmarkScale: CGFloat = 22.0 / 260.0
}

/// The "buddy" wordmark: exact vector transcription of the approved Figma export
/// (`.agents` design pass) — the "u" is replaced by a soft `>_` mark, echoing codex's own
/// prompt glyph without borrowing its cloud shape or color. Native canvas is 700x260pt;
/// callers scale the whole group via `.frame`.
struct BuddyWordmark: View {
    var body: some View {
        ZStack {
            BuddyLettersShape().fill(Theme.brandInk)
            LinearGradient(
                colors: [Theme.brandMarkStart, Theme.brandMarkEnd],
                startPoint: Self.gradientStart,
                endPoint: Self.gradientEnd
            )
            // Masking the gradient (rather than stroking/filling each shape with it) keeps the
            // chevron and the underscore on one shared gradient axis, so the ramp reads as
            // continuous across both instead of restarting inside the tiny underscore.
            .mask {
                ZStack {
                    BuddyChevronShape()
                        .stroke(style: StrokeStyle(lineWidth: 16, lineCap: .round, lineJoin: .round))
                    RoundedRectangle(cornerRadius: 4.5)
                        .frame(width: 17.856, height: 9)
                        .position(x: 164 + 17.856 / 2, y: 128 + 9 / 2)
                }
            }
        }
        .frame(width: 700, height: 260)
    }

    // The mark occupies only x 126-182 of the 700x260 canvas, so the gradient axis is pinned to
    // that box in unit space. Plain `.topLeading`/`.bottomTrailing` would spread the ramp across
    // the full canvas and leave the mark showing one nearly flat slice of it.
    private static let gradientStart = UnitPoint(x: 126 / 700, y: 100 / 260)
    private static let gradientEnd = UnitPoint(x: 182 / 700, y: 166 / 260)
}

/// "b" + "ddy", transcribed 1:1 from the exported SVG path data (see `.agents/tray-logo-mockup.html`
/// history for the design exploration that led here).
private struct BuddyLettersShape: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()

        // b
        path.move(to: CGPoint(x: 78.64, y: 108.88))
        path.addCurve(to: CGPoint(x: 93.04, y: 112.36), control1: CGPoint(x: 83.92, y: 108.88), control2: CGPoint(x: 88.72, y: 110.04))
        path.addCurve(to: CGPoint(x: 103.24, y: 122.68), control1: CGPoint(x: 97.36, y: 114.68), control2: CGPoint(x: 100.76, y: 118.12))
        path.addCurve(to: CGPoint(x: 107.08, y: 139.6), control1: CGPoint(x: 105.8, y: 127.24), control2: CGPoint(x: 107.08, y: 132.88))
        path.addCurve(to: CGPoint(x: 103.0, y: 156.64), control1: CGPoint(x: 107.08, y: 146.24), control2: CGPoint(x: 105.72, y: 151.92))
        path.addCurve(to: CGPoint(x: 91.72, y: 167.32), control1: CGPoint(x: 100.36, y: 161.28), control2: CGPoint(x: 96.6, y: 164.84))
        path.addCurve(to: CGPoint(x: 74.56, y: 170.92), control1: CGPoint(x: 86.84, y: 169.72), control2: CGPoint(x: 81.12, y: 170.92))
        path.addCurve(to: CGPoint(x: 61.12, y: 169.0), control1: CGPoint(x: 69.28, y: 170.92), control2: CGPoint(x: 64.8, y: 170.28))
        path.addCurve(to: CGPoint(x: 51.76, y: 164.44), control1: CGPoint(x: 57.44, y: 167.72), control2: CGPoint(x: 54.32, y: 166.2))
        path.addCurve(to: CGPoint(x: 47.8, y: 160.24), control1: CGPoint(x: 50.08, y: 163.24), control2: CGPoint(x: 48.76, y: 161.84))
        path.addCurve(to: CGPoint(x: 46.48, y: 154.84), control1: CGPoint(x: 46.92, y: 158.64), control2: CGPoint(x: 46.48, y: 156.84))
        path.addLine(to: CGPoint(x: 46.48, y: 117.4))
        path.addLine(to: CGPoint(x: 66.76, y: 117.4))
        path.addLine(to: CGPoint(x: 66.76, y: 153.04))
        path.addCurve(to: CGPoint(x: 69.88, y: 154.36), control1: CGPoint(x: 67.64, y: 153.52), control2: CGPoint(x: 68.68, y: 153.96))
        path.addCurve(to: CGPoint(x: 74.32, y: 154.84), control1: CGPoint(x: 71.16, y: 154.68), control2: CGPoint(x: 72.64, y: 154.84))
        path.addCurve(to: CGPoint(x: 82.96, y: 151.0), control1: CGPoint(x: 77.92, y: 154.84), control2: CGPoint(x: 80.8, y: 153.56))
        path.addCurve(to: CGPoint(x: 86.2, y: 139.6), control1: CGPoint(x: 85.12, y: 148.36), control2: CGPoint(x: 86.2, y: 144.56))
        path.addCurve(to: CGPoint(x: 83.08, y: 128.44), control1: CGPoint(x: 86.2, y: 134.48), control2: CGPoint(x: 85.16, y: 130.76))
        path.addCurve(to: CGPoint(x: 74.8, y: 124.84), control1: CGPoint(x: 81.08, y: 126.04), control2: CGPoint(x: 78.32, y: 124.84))
        path.addCurve(to: CGPoint(x: 68.44, y: 126.16), control1: CGPoint(x: 72.4, y: 124.84), control2: CGPoint(x: 70.28, y: 125.28))
        path.addCurve(to: CGPoint(x: 63.64, y: 129.04), control1: CGPoint(x: 66.6, y: 127.04), control2: CGPoint(x: 65.0, y: 128.0))
        path.addLine(to: CGPoint(x: 63.64, y: 112.6))
        path.addCurve(to: CGPoint(x: 70.24, y: 110.08), control1: CGPoint(x: 65.48, y: 111.64), control2: CGPoint(x: 67.68, y: 110.8))
        path.addCurve(to: CGPoint(x: 78.64, y: 108.88), control1: CGPoint(x: 72.8, y: 109.28), control2: CGPoint(x: 75.6, y: 108.88))
        path.closeSubpath()
        path.move(to: CGPoint(x: 66.88, y: 122.32))
        path.addLine(to: CGPoint(x: 46.48, y: 122.32))
        path.addLine(to: CGPoint(x: 46.48, y: 90.76))
        path.addCurve(to: CGPoint(x: 50.2, y: 90.04), control1: CGPoint(x: 47.36, y: 90.52), control2: CGPoint(x: 48.6, y: 90.28))
        path.addCurve(to: CGPoint(x: 55.6, y: 89.68), control1: CGPoint(x: 51.88, y: 89.8), control2: CGPoint(x: 53.68, y: 89.68))
        path.addCurve(to: CGPoint(x: 64.24, y: 91.72), control1: CGPoint(x: 59.6, y: 89.68), control2: CGPoint(x: 62.48, y: 90.36))
        path.addCurve(to: CGPoint(x: 66.88, y: 99.28), control1: CGPoint(x: 66.0, y: 93.0), control2: CGPoint(x: 66.88, y: 95.52))
        path.addLine(to: CGPoint(x: 66.88, y: 122.32))
        path.closeSubpath()

        // ddy
        path.move(to: CGPoint(x: 228.96, y: 153.28))
        path.addLine(to: CGPoint(x: 228.96, y: 118.48))
        path.addLine(to: CGPoint(x: 249.24, y: 118.48))
        path.addLine(to: CGPoint(x: 249.24, y: 156.16))
        path.addCurve(to: CGPoint(x: 247.92, y: 161.32), control1: CGPoint(x: 249.24, y: 158.16), control2: CGPoint(x: 248.8, y: 159.88))
        path.addCurve(to: CGPoint(x: 243.96, y: 165.16), control1: CGPoint(x: 247.12, y: 162.76), control2: CGPoint(x: 245.8, y: 164.04))
        path.addCurve(to: CGPoint(x: 234.96, y: 169.12), control1: CGPoint(x: 241.64, y: 166.68), control2: CGPoint(x: 238.64, y: 168.0))
        path.addCurve(to: CGPoint(x: 222.36, y: 170.92), control1: CGPoint(x: 231.28, y: 170.32), control2: CGPoint(x: 227.08, y: 170.92))
        path.addCurve(to: CGPoint(x: 204.48, y: 167.44), control1: CGPoint(x: 215.4, y: 170.92), control2: CGPoint(x: 209.44, y: 169.76))
        path.addCurve(to: CGPoint(x: 193.08, y: 157.12), control1: CGPoint(x: 199.52, y: 165.12), control2: CGPoint(x: 195.72, y: 161.68))
        path.addCurve(to: CGPoint(x: 189.12, y: 140.08), control1: CGPoint(x: 190.44, y: 152.48), control2: CGPoint(x: 189.12, y: 146.8))
        path.addCurve(to: CGPoint(x: 193.2, y: 122.68), control1: CGPoint(x: 189.12, y: 133.04), control2: CGPoint(x: 190.48, y: 127.24))
        path.addCurve(to: CGPoint(x: 204.24, y: 112.36), control1: CGPoint(x: 195.92, y: 118.04), control2: CGPoint(x: 199.6, y: 114.6))
        path.addCurve(to: CGPoint(x: 219.96, y: 108.88), control1: CGPoint(x: 208.96, y: 110.04), control2: CGPoint(x: 214.2, y: 108.88))
        path.addCurve(to: CGPoint(x: 227.28, y: 109.72), control1: CGPoint(x: 222.76, y: 108.88), control2: CGPoint(x: 225.2, y: 109.16))
        path.addCurve(to: CGPoint(x: 232.44, y: 111.64), control1: CGPoint(x: 229.36, y: 110.28), control2: CGPoint(x: 231.08, y: 110.92))
        path.addLine(to: CGPoint(x: 232.44, y: 128.2))
        path.addCurve(to: CGPoint(x: 228.36, y: 125.92), control1: CGPoint(x: 231.4, y: 127.4), control2: CGPoint(x: 230.04, y: 126.64))
        path.addCurve(to: CGPoint(x: 222.84, y: 124.84), control1: CGPoint(x: 226.76, y: 125.2), control2: CGPoint(x: 224.92, y: 124.84))
        path.addCurve(to: CGPoint(x: 215.76, y: 126.52), control1: CGPoint(x: 220.12, y: 124.84), control2: CGPoint(x: 217.76, y: 125.4))
        path.addCurve(to: CGPoint(x: 211.32, y: 131.56), control1: CGPoint(x: 213.84, y: 127.64), control2: CGPoint(x: 212.36, y: 129.32))
        path.addCurve(to: CGPoint(x: 209.76, y: 140.08), control1: CGPoint(x: 210.28, y: 133.8), control2: CGPoint(x: 209.76, y: 136.64))
        path.addCurve(to: CGPoint(x: 213.0, y: 151.24), control1: CGPoint(x: 209.76, y: 145.12), control2: CGPoint(x: 210.84, y: 148.84))
        path.addCurve(to: CGPoint(x: 222.24, y: 154.84), control1: CGPoint(x: 215.24, y: 153.64), control2: CGPoint(x: 218.32, y: 154.84))
        path.addCurve(to: CGPoint(x: 226.2, y: 154.36), control1: CGPoint(x: 223.68, y: 154.84), control2: CGPoint(x: 225.0, y: 154.68))
        path.addCurve(to: CGPoint(x: 228.96, y: 153.28), control1: CGPoint(x: 227.4, y: 153.96), control2: CGPoint(x: 228.32, y: 153.6))
        path.closeSubpath()
        path.move(to: CGPoint(x: 249.24, y: 122.32))
        path.addLine(to: CGPoint(x: 228.84, y: 122.32))
        path.addLine(to: CGPoint(x: 228.84, y: 90.76))
        path.addCurve(to: CGPoint(x: 232.56, y: 90.04), control1: CGPoint(x: 229.72, y: 90.52), control2: CGPoint(x: 230.96, y: 90.28))
        path.addCurve(to: CGPoint(x: 237.96, y: 89.68), control1: CGPoint(x: 234.24, y: 89.8), control2: CGPoint(x: 236.04, y: 89.68))
        path.addCurve(to: CGPoint(x: 246.6, y: 91.72), control1: CGPoint(x: 241.96, y: 89.68), control2: CGPoint(x: 244.84, y: 90.36))
        path.addCurve(to: CGPoint(x: 249.24, y: 99.28), control1: CGPoint(x: 248.36, y: 93.0), control2: CGPoint(x: 249.24, y: 95.52))
        path.addLine(to: CGPoint(x: 249.24, y: 122.32))
        path.closeSubpath()
        path.move(to: CGPoint(x: 299.272, y: 153.28))
        path.addLine(to: CGPoint(x: 299.272, y: 118.48))
        path.addLine(to: CGPoint(x: 319.552, y: 118.48))
        path.addLine(to: CGPoint(x: 319.552, y: 156.16))
        path.addCurve(to: CGPoint(x: 318.232, y: 161.32), control1: CGPoint(x: 319.552, y: 158.16), control2: CGPoint(x: 319.112, y: 159.88))
        path.addCurve(to: CGPoint(x: 314.272, y: 165.16), control1: CGPoint(x: 317.432, y: 162.76), control2: CGPoint(x: 316.112, y: 164.04))
        path.addCurve(to: CGPoint(x: 305.272, y: 169.12), control1: CGPoint(x: 311.952, y: 166.68), control2: CGPoint(x: 308.952, y: 168.0))
        path.addCurve(to: CGPoint(x: 292.672, y: 170.92), control1: CGPoint(x: 301.592, y: 170.32), control2: CGPoint(x: 297.392, y: 170.92))
        path.addCurve(to: CGPoint(x: 274.792, y: 167.44), control1: CGPoint(x: 285.712, y: 170.92), control2: CGPoint(x: 279.752, y: 169.76))
        path.addCurve(to: CGPoint(x: 263.392, y: 157.12), control1: CGPoint(x: 269.832, y: 165.12), control2: CGPoint(x: 266.032, y: 161.68))
        path.addCurve(to: CGPoint(x: 259.432, y: 140.08), control1: CGPoint(x: 260.752, y: 152.48), control2: CGPoint(x: 259.432, y: 146.8))
        path.addCurve(to: CGPoint(x: 263.512, y: 122.68), control1: CGPoint(x: 259.432, y: 133.04), control2: CGPoint(x: 260.792, y: 127.24))
        path.addCurve(to: CGPoint(x: 274.552, y: 112.36), control1: CGPoint(x: 266.232, y: 118.04), control2: CGPoint(x: 269.912, y: 114.6))
        path.addCurve(to: CGPoint(x: 290.272, y: 108.88), control1: CGPoint(x: 279.272, y: 110.04), control2: CGPoint(x: 284.512, y: 108.88))
        path.addCurve(to: CGPoint(x: 297.592, y: 109.72), control1: CGPoint(x: 293.072, y: 108.88), control2: CGPoint(x: 295.512, y: 109.16))
        path.addCurve(to: CGPoint(x: 302.752, y: 111.64), control1: CGPoint(x: 299.672, y: 110.28), control2: CGPoint(x: 301.392, y: 110.92))
        path.addLine(to: CGPoint(x: 302.752, y: 128.2))
        path.addCurve(to: CGPoint(x: 298.672, y: 125.92), control1: CGPoint(x: 301.712, y: 127.4), control2: CGPoint(x: 300.352, y: 126.64))
        path.addCurve(to: CGPoint(x: 293.152, y: 124.84), control1: CGPoint(x: 297.072, y: 125.2), control2: CGPoint(x: 295.232, y: 124.84))
        path.addCurve(to: CGPoint(x: 286.072, y: 126.52), control1: CGPoint(x: 290.432, y: 124.84), control2: CGPoint(x: 288.072, y: 125.4))
        path.addCurve(to: CGPoint(x: 281.632, y: 131.56), control1: CGPoint(x: 284.152, y: 127.64), control2: CGPoint(x: 282.672, y: 129.32))
        path.addCurve(to: CGPoint(x: 280.072, y: 140.08), control1: CGPoint(x: 280.592, y: 133.8), control2: CGPoint(x: 280.072, y: 136.64))
        path.addCurve(to: CGPoint(x: 283.312, y: 151.24), control1: CGPoint(x: 280.072, y: 145.12), control2: CGPoint(x: 281.152, y: 148.84))
        path.addCurve(to: CGPoint(x: 292.552, y: 154.84), control1: CGPoint(x: 285.552, y: 153.64), control2: CGPoint(x: 288.632, y: 154.84))
        path.addCurve(to: CGPoint(x: 296.512, y: 154.36), control1: CGPoint(x: 293.992, y: 154.84), control2: CGPoint(x: 295.312, y: 154.68))
        path.addCurve(to: CGPoint(x: 299.272, y: 153.28), control1: CGPoint(x: 297.712, y: 153.96), control2: CGPoint(x: 298.632, y: 153.6))
        path.closeSubpath()
        path.move(to: CGPoint(x: 319.552, y: 122.32))
        path.addLine(to: CGPoint(x: 299.152, y: 122.32))
        path.addLine(to: CGPoint(x: 299.152, y: 90.76))
        path.addCurve(to: CGPoint(x: 302.872, y: 90.04), control1: CGPoint(x: 300.032, y: 90.52), control2: CGPoint(x: 301.272, y: 90.28))
        path.addCurve(to: CGPoint(x: 308.272, y: 89.68), control1: CGPoint(x: 304.552, y: 89.8), control2: CGPoint(x: 306.352, y: 89.68))
        path.addCurve(to: CGPoint(x: 316.912, y: 91.72), control1: CGPoint(x: 312.272, y: 89.68), control2: CGPoint(x: 315.152, y: 90.36))
        path.addCurve(to: CGPoint(x: 319.552, y: 99.28), control1: CGPoint(x: 318.672, y: 93.0), control2: CGPoint(x: 319.552, y: 95.52))
        path.addLine(to: CGPoint(x: 319.552, y: 122.32))
        path.closeSubpath()
        path.move(to: CGPoint(x: 341.385, y: 154.12))
        path.addCurve(to: CGPoint(x: 337.545, y: 144.76), control1: CGPoint(x: 340.185, y: 151.4), control2: CGPoint(x: 338.905, y: 148.28))
        path.addCurve(to: CGPoint(x: 333.225, y: 131.92), control1: CGPoint(x: 336.185, y: 141.16), control2: CGPoint(x: 334.745, y: 136.88))
        path.addCurve(to: CGPoint(x: 328.425, y: 113.68), control1: CGPoint(x: 331.785, y: 126.88), control2: CGPoint(x: 330.185, y: 120.8))
        path.addCurve(to: CGPoint(x: 332.865, y: 110.8), control1: CGPoint(x: 329.545, y: 112.56), control2: CGPoint(x: 331.025, y: 111.6))
        path.addCurve(to: CGPoint(x: 339.105, y: 109.48), control1: CGPoint(x: 334.785, y: 109.92), control2: CGPoint(x: 336.865, y: 109.48))
        path.addCurve(to: CGPoint(x: 346.065, y: 111.28), control1: CGPoint(x: 341.905, y: 109.48), control2: CGPoint(x: 344.225, y: 110.08))
        path.addCurve(to: CGPoint(x: 350.145, y: 117.76), control1: CGPoint(x: 347.905, y: 112.4), control2: CGPoint(x: 349.265, y: 114.56))
        path.addCurve(to: CGPoint(x: 354.945, y: 134.44), control1: CGPoint(x: 351.745, y: 123.36), control2: CGPoint(x: 353.345, y: 128.92))
        path.addCurve(to: CGPoint(x: 359.625, y: 150.76), control1: CGPoint(x: 356.545, y: 139.88), control2: CGPoint(x: 358.105, y: 145.32))
        path.addLine(to: CGPoint(x: 360.105, y: 150.76))
        path.addCurve(to: CGPoint(x: 363.825, y: 138.64), control1: CGPoint(x: 361.385, y: 147.08), control2: CGPoint(x: 362.625, y: 143.04))
        path.addCurve(to: CGPoint(x: 367.305, y: 125.2), control1: CGPoint(x: 365.105, y: 134.24), control2: CGPoint(x: 366.265, y: 129.76))
        path.addCurve(to: CGPoint(x: 370.065, y: 111.76), control1: CGPoint(x: 368.425, y: 120.56), control2: CGPoint(x: 369.345, y: 116.08))
        path.addCurve(to: CGPoint(x: 374.265, y: 110.08), control1: CGPoint(x: 371.425, y: 111.04), control2: CGPoint(x: 372.825, y: 110.48))
        path.addCurve(to: CGPoint(x: 378.825, y: 109.48), control1: CGPoint(x: 375.705, y: 109.68), control2: CGPoint(x: 377.225, y: 109.48))
        path.addCurve(to: CGPoint(x: 386.145, y: 111.28), control1: CGPoint(x: 381.625, y: 109.48), control2: CGPoint(x: 384.065, y: 110.08))
        path.addCurve(to: CGPoint(x: 389.265, y: 117.52), control1: CGPoint(x: 388.225, y: 112.48), control2: CGPoint(x: 389.265, y: 114.56))
        path.addCurve(to: CGPoint(x: 388.305, y: 126.4), control1: CGPoint(x: 389.265, y: 120.08), control2: CGPoint(x: 388.945, y: 123.04))
        path.addCurve(to: CGPoint(x: 385.665, y: 137.08), control1: CGPoint(x: 387.665, y: 129.76), control2: CGPoint(x: 386.785, y: 133.32))
        path.addCurve(to: CGPoint(x: 381.825, y: 148.48), control1: CGPoint(x: 384.545, y: 140.84), control2: CGPoint(x: 383.265, y: 144.64))
        path.addCurve(to: CGPoint(x: 377.265, y: 159.4), control1: CGPoint(x: 380.385, y: 152.24), control2: CGPoint(x: 378.865, y: 155.88))
        path.addCurve(to: CGPoint(x: 372.465, y: 168.76), control1: CGPoint(x: 375.665, y: 162.84), control2: CGPoint(x: 374.065, y: 165.96))
        path.addCurve(to: CGPoint(x: 359.985, y: 186.04), control1: CGPoint(x: 367.985, y: 176.52), control2: CGPoint(x: 363.825, y: 182.28))
        path.addCurve(to: CGPoint(x: 348.345, y: 191.68), control1: CGPoint(x: 356.145, y: 189.8), control2: CGPoint(x: 352.265, y: 191.68))
        path.addCurve(to: CGPoint(x: 340.305, y: 188.68), control1: CGPoint(x: 345.145, y: 191.68), control2: CGPoint(x: 342.465, y: 190.68))
        path.addCurve(to: CGPoint(x: 336.465, y: 180.88), control1: CGPoint(x: 338.145, y: 186.68), control2: CGPoint(x: 336.865, y: 184.08))
        path.addCurve(to: CGPoint(x: 341.505, y: 176.2), control1: CGPoint(x: 338.145, y: 179.44), control2: CGPoint(x: 339.825, y: 177.88))
        path.addCurve(to: CGPoint(x: 346.545, y: 170.8), control1: CGPoint(x: 343.265, y: 174.52), control2: CGPoint(x: 344.945, y: 172.72))
        path.addCurve(to: CGPoint(x: 350.865, y: 165.28), control1: CGPoint(x: 348.145, y: 168.96), control2: CGPoint(x: 349.585, y: 167.12))
        path.addCurve(to: CGPoint(x: 346.425, y: 162.52), control1: CGPoint(x: 349.505, y: 164.88), control2: CGPoint(x: 348.025, y: 163.96))
        path.addCurve(to: CGPoint(x: 341.385, y: 154.12), control1: CGPoint(x: 344.905, y: 161.0), control2: CGPoint(x: 343.225, y: 158.2))
        path.closeSubpath()

        return path
    }
}

/// The `>_` mark standing in for "u". Also reused standalone as the menu bar status item icon
/// (see `AppDelegate.makeStatusBarIcon`).
struct BuddyChevronShape: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        path.move(to: CGPoint(x: 126, y: 104))
        path.addLine(to: CGPoint(x: 149.808, y: 132.8))
        path.addLine(to: CGPoint(x: 126, y: 161.6))
        return path
    }
}
