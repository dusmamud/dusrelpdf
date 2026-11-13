package io.github.dusmamud.dusrelpdf_flutter

import io.flutter.embedding.engine.plugins.FlutterPlugin

/** DusrelpdfFlutterPlugin.
 *
 * No-op registrant: this is an ffi plugin, so there is no MethodChannel
 * and no platform-side API. All calls go from Dart to the native
 * library (libdusrelpdf.so, bundled via the Maven Central AAR or the
 * local jniLibs fallback — see android/build.gradle) through dart:ffi.
 * This class exists only because the Flutter tooling requires a
 * plugin class for every platform entry in pubspec.yaml.
 */
class DusrelpdfFlutterPlugin : FlutterPlugin {
    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        // Nothing to register: the Dart side opens libdusrelpdf.so
        // directly (DusrelpdfFlutter.defaultLibraryPath()).
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        // Nothing to clean up.
    }
}
