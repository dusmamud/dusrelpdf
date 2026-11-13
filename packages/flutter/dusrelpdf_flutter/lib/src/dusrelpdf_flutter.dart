import 'dart:io';

import 'package:dusrelpdf/dusrelpdf.dart';

/// The Flutter entry point for dusrelpdf.
///
/// A [Dusrelpdf] that loads the native library the way this plugin
/// bundles it on each platform — no `libraryPath` needed in an app:
///
/// | Platform        | How the library is bundled & found                 |
/// | --------------- | -------------------------------------------------- |
/// | Android         | `.so` from the AAR's `jniLibs` (or the local       |
/// |                 | `android/src/main/jniLibs` fallback); loaded by    |
/// |                 | its bare name `libdusrelpdf.so`.                   |
/// | iOS / macOS     | `Dusrelpdf.xcframework` embedded in the app        |
/// |                 | bundle (`.../Frameworks/Dusrelpdf.framework/`).   |
/// | Linux           | `libdusrelpdf.so` bundled next to the app binary   |
/// |                 | (Flutter's `bundled_libraries`); bare name.       |
/// | Windows         | `dusrelpdf.dll` bundled next to the `.exe`;        |
/// |                 | bare name.                                           |
///
/// Pass an explicit [libraryPath] to override (e.g. in tests, or to
/// point at a locally built library). The `DUSRELPDFFI_LIB`
/// environment variable also overrides, same as in `dusrelpdf`.
///
/// NOTE: until the first v0.1.0 release, the AAR / XCFramework /
/// prebuilt archives do not exist yet (see README.md), so on a real
/// device today only the local-build paths work.
class DusrelpdfFlutter extends Dusrelpdf {
  DusrelpdfFlutter({String? libraryPath})
      : super(libraryPath: libraryPath ?? defaultLibraryPath());

  /// The library path this plugin uses on the current platform when
  /// no explicit path is given. Pure Dart (no native library needed),
  /// so it is unit-testable on any host.
  static String defaultLibraryPath() {
    if (Platform.isIOS || Platform.isMacOS) {
      // The XCFramework is embedded in the app bundle; the framework
      // binary lives at <App>.app/Frameworks/Dusrelpdf.framework/
      // Dusrelpdf, next to the app executable. (The usual
      // `DynamicLibrary.process()` alternative is not available here:
      // `Dusrelpdf` loads by path, so the framework path is used.)
      final executable = Platform.resolvedExecutable;
      final appDir = File(executable).parent.path;
      return '$appDir/Frameworks/Dusrelpdf.framework/Dusrelpdf';
    }
    if (Platform.isWindows) return 'dusrelpdf.dll';
    // Android + Linux: the bundled .so is found by the OS loader
    // under its bare name.
    return 'libdusrelpdf.so';
  }
}
