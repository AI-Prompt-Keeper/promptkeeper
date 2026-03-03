// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "PromptKeeper",
    platforms: [
        .iOS(.v15),
        .macOS(.v12)
    ],
    products: [
        .library(name: "PromptKeeper", targets: ["PromptKeeper"])
    ],
    targets: [
        .target(
            name: "PromptKeeper",
            path: "Sources/PromptKeeper"
        ),
        .testTarget(
            name: "PromptKeeperTests",
            dependencies: ["PromptKeeper"],
            path: "Tests/PromptKeeperTests"
        )
    ]
)
