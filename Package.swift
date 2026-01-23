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
            url: "https://github.com/cooklang/cooklang-rs/releases/download/v0.17.14/CooklangParserFFI.xcframework.zip",
            checksum: "de8310c84cd2376babe8d6e19239db86195e115131ce5d52ab1d59716d73c5ff"),
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
