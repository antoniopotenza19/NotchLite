// swift-tools-version: 5.9
import PackageDescription
let package = Package(name: "CompanionProtocol", products: [.library(name: "CompanionProtocol", targets: ["CompanionProtocol"])], targets: [
    .target(name: "CompanionProtocol", path: "NotchLiteCompanion/App", exclude: ["NotchLiteCompanionApp.swift", "PinnedClient.swift", "CompanionModel.swift", "ContentView.swift", "CameraScanner.swift"], sources: ["PairQR.swift"]),
    .testTarget(name: "CompanionProtocolTests", dependencies: ["CompanionProtocol"], path: "Tests")
])
