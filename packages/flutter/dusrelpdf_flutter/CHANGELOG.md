## 0.1.0

- First release (unpublished — the package is not on pub.dev yet).
- Flutter plugin scaffolding for the pure-Dart `dusrelpdf` package:
  `flutter.plugin.platforms` with `ffiPlugin: true` for android, ios,
  macos, linux and windows.
- Android: Gradle dependency on the future Maven Central AAR
  `io.github.dusmamud:dusrelpdf:0.1.0` (see `android/build.gradle` —
  commented out until the AAR exists) with a local `jniLibs` fallback.
- iOS/macOS: podspecs downloading the future `Dusrelpdf.xcframework`
  from the first GitHub Release, with a SHA-256 placeholder and a
  local-`Frameworks/` fallback.
- Linux/Windows: CMake downloading the future prebuilt archives from
  the first GitHub Release (pinned URL + SHA-256 placeholder) with a
  local-build fallback.
- `DusrelpdfFlutter` wrapper: re-exports the whole `dusrelpdf` API and
  resolves the bundled native library per platform.
