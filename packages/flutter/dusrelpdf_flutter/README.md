# dusrelpdf_flutter — Flutter plugin wiring for dusrelpdf

Flutter plugin package for [dusrelpdf](https://github.com/dusmamud/dusrelpdf):
an offline PDF toolkit (merge, split, extract, reorder, rotate, watermark,
page numbers, images-to-PDF, signature-image placement) with its core in
Rust. **Not published to pub.dev.**

This package holds **only the native wiring**. All PDF logic lives in the
pure-Dart package [`dusrelpdf`](../dusrelpdf/) (dart:ffi over the Rust
core); this package re-exports it and bundles the native library
(`libdusrelpdf`) into a Flutter app on each platform, so an app no longer
has to build the Rust cdylib or bundle it by hand.

## Install

```sh
flutter pub add dusrelpdf_flutter
```

Until the first pub.dev publish, depend on it by path instead (from your
app's `pubspec.yaml`):

```yaml
dependencies:
  dusrelpdf_flutter:
    path: packages/flutter/dusrelpdf_flutter
```

## Use it

```dart
import 'package:dusrelpdf_flutter/dusrelpdf_flutter.dart';

final pdf = DusrelpdfFlutter();

// Merge two PDFs; null options = Rust defaults.
final merged = pdf.merge([a, b]);

// Summary without modifying the PDF.
final info = pdf.getPdfInfo(merged);
print(info.pageCount); // e.g. 8

// Errors are DusrelException with a stable code:
try {
  pdf.extractPagesRange(doc, '9');
} on DusrelException catch (e) {
  print(e.code); // INVALID_RANGE
  print(e.messageEn);
  print(e.messageHi);
}
```

`DusrelpdfFlutter` is a `Dusrelpdf` — the whole API of the `dusrelpdf`
package is re-exported (`merge`, `split`, `extractPages`, `removePages`,
`reorderPages`, `rotate`, `getPdfInfo`, `imagesToPdf`, `addWatermark`,
`addPageNumbers`, `placeSignature`, `bulkExtract`, …). See
[`dusrelpdf`'s README](../dusrelpdf/README.md) for the full API table.

## How the native library gets bundled (per platform)

| Platform | Bundled as | Source |
| -------- | ---------- | ------ |
| Android | `libdusrelpdf.so` per ABI, from the AAR's `jni/` | Maven Central AAR (future) or local `jniLibs` (today) |
| iOS | `Dusrelpdf.xcframework` embedded in the app | GitHub Release asset (future) or local `ios/Frameworks/` (today) |
| macOS | `Dusrelpdf.xcframework` embedded in the app | GitHub Release asset (future) or local `macos/Frameworks/` (today) |
| Linux | `libdusrelpdf.so` next to the app binary | GitHub Release archive (future) or local build (today) |
| Windows | `dusrelpdf.dll` next to the `.exe` | GitHub Release archive (future) or local build (today) |

The Dart side finds the library automatically
(`DusrelpdfFlutter.defaultLibraryPath()`): bare name `libdusrelpdf.so` /
`dusrelpdf.dll` on Android/Linux/Windows, and
`<App>.app/Frameworks/Dusrelpdf.framework/Dusrelpdf` on iOS/macOS.
Pass an explicit `libraryPath` (or set `DUSRELPDFFI_LIB`) to override.

There is no MethodChannel: the plugin classes on each platform
(`DusrelpdfFlutterPlugin` in Kotlin/Swift/C++/C) are no-op registrants —
all calls go Dart → dart:ffi → Rust.

## ⚠️ Release assets do not exist yet — today it only works from a local build

**As of v0.1.0, dusrelpdf has never been released or published anywhere:**
no GitHub Release, no Maven Central AAR, no XCFramework, no prebuilt
archives, no pub.dev package. So the "future" sources in the table above
do not exist yet, and the wiring that consumes them is deliberately
fail-loud until the first v0.1.0 release:

- **Android** — `android/build.gradle` contains the dependency
  `api 'io.github.dusmamud:dusrelpdf:0.1.0'`, **commented out** (an
  active dependency on a non-existent AAR would break every Android
  build). After the first Maven Central publish, uncomment that one
  line and delete the `jniLibs` fallback.
- **iOS/macOS** — the podspecs download the pinned asset
  `https://github.com/dusmamud/dusrelpdf/releases/download/v0.1.0/dusrelpdf.xcframework.zip`
  in `prepare_command` and verify its SHA-256. The checksum in the
  podspec is a clearly-marked **placeholder, not a real checksum** —
  `pod install` refuses to download until it is replaced. Two-phase
  step after the first release: publish the XCFramework zip in the
  v0.1.0 GitHub Release, run `shasum -a 256 dusrelpdf.xcframework.zip`,
  and put the output in both
  `ios/dusrelpdf_flutter.podspec` and `macos/dusrelpdf_flutter.podspec`.
  (One XCFramework with iOS device + simulator + macOS slices.)
- **Linux** — `linux/CMakeLists.txt` downloads the pinned archive
  `dusrelpdf-linux-<arch>.tar.gz` from the v0.1.0 GitHub Release and
  verifies its SHA-256. The checksum is a clearly-marked
  **placeholder** — the CMake configure step fails with instructions
  until it is replaced (after the release: `sha256sum` the archive and
  set `DUSRELPDFFI_LINUX_SHA256`).
- **Windows** — `windows/CMakeLists.txt`, same story with
  `dusrelpdf-windows-<arch>.zip` and `DUSRELPDFFI_WINDOWS_SHA256`
  (after the release: `(Get-FileHash …).Hash`).

### Working from a local build today

Build the Rust cdylib once, from the repo root:

```sh
cargo build -p dusrelpdf-ffi --release
# Linux:   target/release/libdusrelpdf.so
# macOS:   target/release/libdusrelpdf.dylib
# Windows: target/release/dusrelpdf.dll
```

Then, per platform:

- **Android** — build per ABI and copy into the plugin:
  ```sh
  cargo ndk -t armeabi-v7a -t arm64-v8a -t x86_64 \
    build -p dusrelpdf-ffi --release
  # copy target/<abi>/release/libdusrelpdf.so to
  # packages/flutter/dusrelpdf_flutter/android/src/main/jniLibs/<abi>/
  ```
- **iOS/macOS** — build an XCFramework from the cdylib (e.g. with
  `xcodebuild -create-xcframework`) and place it at
  `packages/flutter/dusrelpdf_flutter/ios/Frameworks/Dusrelpdf.xcframework`
  (or `macos/Frameworks/` for macOS). The podspec copies it into place
  at `pod install` time. (No local XCFramework is committed.)
- **Linux** — nothing to do: CMake finds `target/release/libdusrelpdf.so`
  in the repo layout automatically. Or pass
  `-DDUSRELPDFFI_LIB=/path/to/libdusrelpdf.so` / set `DUSRELPDFFI_LIB`,
  or drop the `.so` in `linux/third_party/`.
- **Windows** — same: CMake finds `target/release/dusrelpdf.dll`
  automatically, or use `DUSRELPDFFI_LIB` / `windows/third_party/`.

## Tests

```sh
# Needs the cdylib built first: cargo build -p dusrelpdf-ffi --release
dart test test/dusrelpdf_flutter_test.dart   # 6 dart tests

# Widget test — REQUIRES the Flutter SDK:
flutter test test/dusrelpdf_flutter_widget_test.dart
flutter analyze
```

**What was NOT run here:** the Flutter SDK is not installed on this
machine, so no widget-level test was run and `flutter analyze` /
`flutter test` could not be executed. The two pure-Dart tests that do
not need the native library (`defaultLibraryPath`, re-export check)
were written to be runnable with `dart test` once a Dart SDK is
available, but no Dart SDK is installed here either, so **no test in
this package has been executed on this machine** — `dart test`,
`dart analyze`, `flutter test` and `flutter analyze` are all unverified
here. The pubspec YAML was validated (parses cleanly), and the
CMake/podspec/Gradle files follow the `flutter create --template=plugin`
layout, but they have not been through a real `flutter build` on any
platform — that is first-release verification work.

Do not run plain `dart test` on the whole `test/` directory: the widget
test imports `package:flutter_test` and only compiles under Flutter.

Fixtures (`test/testdata/three.pdf`, `test/testdata/five.pdf`) are copies
of the pure-Dart package's fixtures (real PDFs generated by the repo's
own CLI).

## Honest limits

- Same v0.1.0 core limits as `dusrelpdf`: no rendering, no tolerant
  repair, merge drops original outlines (rebuilt flat), watermark /
  page-number text is ASCII only, signature is a raster image, not a
  cryptographic signature.
- Whole documents live in memory; no streaming.
- The `dusrelpdf` path dependency in `pubspec.yaml` must become the
  hosted dependency `dusrelpdf: ^0.1.0` before the first pub.dev
  publish (that requires `dusrelpdf` to be published first).
- No `example/` app is shipped yet — a minimal example app is
  first-release follow-up work, not part of this scaffolding.
- `.dylib` / `.dll` builds and the Xcode/Gradle builds were not
  produced on this Linux VM — the wiring follows Flutter's plugin
  template, but iOS/macOS/Android/Windows builds are unverified here.
