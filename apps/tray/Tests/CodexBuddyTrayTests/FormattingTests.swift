import CodexBuddyFFI
import XCTest

@testable import CodexBuddyTray

final class FormattingTests: XCTestCase {
    private func window(_ minutes: Int64, used: Double) -> UsageWindow {
        UsageWindow(windowMinutes: minutes, usedPercent: used, resetsAt: nil)
    }

    private func account(_ alias: String) -> Account {
        Account(
            alias: alias, email: nil, plan: nil, isActive: false, isRunning: false,
            usage: [], lastUsedAt: nil
        )
    }

    func testWindowLabelBoundary() {
        XCTAssertEqual(window(300, used: 0).label, "5h")
        XCTAssertEqual(window(301, used: 0).label, "7d")
        XCTAssertEqual(window(10080, used: 0).label, "7d")
    }

    func testRemainingPercentClampsAtZero() {
        XCTAssertEqual(window(300, used: 130).remainingPercent, 0)
        XCTAssertEqual(window(300, used: 46).remainingPercent, 54)
    }

    func testTightestPicksTheMostUsedWindow() {
        let windows = [window(300, used: 20), window(10080, used: 80)]
        XCTAssertEqual(windows.tightest?.windowMinutes, 10080)
        XCTAssertEqual(windows.secondary?.windowMinutes, 300)
        XCTAssertNil([window(300, used: 5)].secondary)
        XCTAssertNil([UsageWindow]().tightest)
    }

    func testFiveHourAndWeeklySelection() {
        let windows = [window(10080, used: 1), window(300, used: 2)]
        XCTAssertEqual(windows.fiveHour?.windowMinutes, 300)
        XCTAssertEqual(windows.weekly?.windowMinutes, 10080)
    }

    func testAccountInitial() {
        XCTAssertEqual(account("work").initial, "W")
        XCTAssertEqual(account("").initial, "")
    }

    func testSeverityThresholds() {
        XCTAssertEqual(Theme.Severity(remainingPercent: 50), .plenty)
        XCTAssertEqual(Theme.Severity(remainingPercent: 49.9), .low)
        XCTAssertEqual(Theme.Severity(remainingPercent: 20), .low)
        XCTAssertEqual(Theme.Severity(remainingPercent: 19.9), .critical)
    }

    func testAccountHueIsStablePerAlias() {
        for alias in ["work", "default", "personal"] {
            XCTAssertEqual(Theme.AccountHue.forAlias(alias), Theme.AccountHue.forAlias(alias))
        }
    }

    func testAppearanceModeRawValueRoundTrip() {
        for mode in AppearanceMode.allCases {
            XCTAssertEqual(AppearanceMode(rawValue: mode.rawValue), mode)
        }
    }
}
