// swift-tools-version: 5.7
import PackageDescription
import class Foundation.ProcessInfo

var package = Package(
    name: "cooklang-rs",
    platforms: [
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
            url: "https://github.com/cooklang/cooklang-rs/releases/download/refs/heads/main/CooklangParserFFI.xcframework.zip",
            checksum: "960f8605fc607edad81317bbca5b1dabeaa387c286df956ea5dab5370b753844"),
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
