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
            url: "https://github.com/cooklang/cooklang-rs/releases/download/v0.17.12/CooklangParserFFI.xcframework.zip",
            checksum: "503e9515d1191cadfe98d87d7449bb5ce35d3f9140ed12b05300e3d95b7c4acb"),
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
