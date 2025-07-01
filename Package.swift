// swift-tools-version: 5.7
import PackageDescription
import class Foundation.ProcessInfo

var package = Package(
    name: "cooklang-rs",
    platforms: [
        .macOS(.v10_15),
        .iOS(.v15),
    ],
    products: [
        .library(
            name: "CooklangParser",
            targets: ["CooklangParser"]),
    ],
    dependencies: [
    ],
    targets: [
        .target(
            name: "CooklangParser",
            path: "swift/Sources/CooklangParser"),
        .testTarget(
            name: "CooklangParserTests",
            dependencies: ["CooklangParser"],
            path: "swift/Tests/CooklangParserTests"),
        .binaryTarget(
            name: "CooklangParserFFI",
            url: "https://github.com/cooklang/cooklang-rs/releases/download/v0.16.2/CooklangParserFFI.xcframework.zip",
            checksum: "91cc92affa9a6e3adf6478aec8ba95b8afbf3eae96607d7c0910bcc8e81105a6"),
    ]
)

let cooklangParserTarget = package.targets.first(where: { $0.name == "CooklangParser" })

if ProcessInfo.processInfo.environment["USE_LOCAL_XCFRAMEWORK"] == nil {
    cooklangParserTarget?.dependencies.append("CooklangParserFFI")
} else {
    package.targets.append(.binaryTarget(
        name: "CooklangParserFFI_local",
        path: "bindings/out/CooklangParserFFI.xcframework"))

    cooklangParserTarget?.dependencies.append("CooklangParserFFI_local")
}
