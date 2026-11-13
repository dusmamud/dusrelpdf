import Flutter
import UIKit

/// No-op registrant: this is an ffi plugin, so there is no
/// MethodChannel and no platform-side API. The Dart side loads the
/// Dusrelpdf.xcframework binary from the app bundle
/// (Frameworks/Dusrelpdf.framework/Dusrelpdf) via dart:ffi.
/// This class exists only because the Flutter tooling requires a
/// plugin class for every platform entry in pubspec.yaml.
public class DusrelpdfFlutterPlugin: NSObject, FlutterPlugin {
  public static func register(with registrar: FlutterPluginRegistrar) {
    // Nothing to register.
  }
}
