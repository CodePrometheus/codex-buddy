// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "CodexBuddyTray",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "CodexBuddyTray", targets: ["CodexBuddyTray"])
    ],
    targets: [
        .binaryTarget(
            name: "CodexBuddyFFIXCFramework",
            path: "CodexBuddyFFI.xcframework"
        ),
        .target(
            name: "CodexBuddyFFI",
            dependencies: ["CodexBuddyFFIXCFramework"]
        ),
        .executableTarget(
            name: "CodexBuddyTray",
            dependencies: ["CodexBuddyFFI"]
        ),
        .testTarget(
            name: "CodexBuddyTrayTests",
            dependencies: ["CodexBuddyTray"]
        ),
    ]
)
