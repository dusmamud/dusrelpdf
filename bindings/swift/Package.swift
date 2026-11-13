// swift-tools-version:5.9
// The swift-tools-version declares the minimum Swift version required to build this package.
import PackageDescription

// MARK: - Release toggle (two-phase XCFramework flow)
//
// Development / CI builds use the LOCAL FFI target below: the UniFFI-generated
// C header in Sources/dusrelpdf_uniffiFFI/ is compiled in, so `swift build`
// works on macOS today with nothing to download. This is the mode CI uses.
//
// For a tagged release the flow is two-phase (see bindings/swift/README.md):
//   1. Build the XCFramework (bindings/swift/build-xcframework.sh, macOS),
//      upload DusrelpdfFFI.xcframework.zip to the tag's GitHub Release.
//   2. Run `swift package compute-checksum` on that zip, paste the real
//      download URL + checksum into the binaryTarget below, flip
//      `useLocalFFI` to `false`, and tag.
//
// The URL and checksum in the binaryTarget below are PLACEHOLDERS — clearly
// marked, never valid, and never to be shipped as-is. Do not invent a
// checksum: it must be computed from the actual uploaded zip.
let useLocalFFI = true

let ffiTarget: Target
if useLocalFFI {
    // Local C target: header-only module built from the generated header.
    // Provides the `dusrelpdf_uniffiFFI` module the generated Swift code
    // imports (`#if canImport(dusrelpdf_uniffiFFI)`). No native library is
    // needed to COMPILE the Swift module; linking against the real Rust
    // staticlib happens when an app links the final product.
    ffiTarget = .target(
        name: "dusrelpdf_uniffiFFI",
        path: "Sources/dusrelpdf_uniffiFFI",
        publicHeadersPath: "."
    )
} else {
    // Release mode: prebuilt XCFramework from the tag's GitHub Release.
    // PLACEHOLDER URL — replace with the real release asset URL.
    // PLACEHOLDER checksum — replace with the output of
    // `swift package compute-checksum DusrelpdfFFI.xcframework.zip`.
    ffiTarget = .binaryTarget(
        name: "dusrelpdf_uniffiFFI",
        url: "https://github.com/dusmamud/dusrelpdf/releases/download/v0.1.0/DusrelpdfFFI.xcframework.zip",
        checksum: "0000000000000000000000000000000000000000000000000000000000000000"
    )
}

let package = Package(
    name: "dusrelpdf",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15),
    ],
    products: [
        .library(
            name: "Dusrelpdf",
            targets: ["Dusrelpdf"]
        ),
    ],
    targets: [
        // The public module consumers import. Contains the UniFFI-generated
        // Swift source (a copy of bindings/swift/dusrelpdf_uniffi.swift —
        // kept in sync by CI; see README.md).
        .target(
            name: "Dusrelpdf",
            dependencies: ["dusrelpdf_uniffiFFI"],
            path: "Sources/Dusrelpdf"
        ),
        ffiTarget,
    ]
)
