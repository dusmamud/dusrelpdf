// No-op registrant: this is an ffi plugin, so there is no MethodChannel
// and no platform-side API. The Dart side loads libdusrelpdf.so (bundled
// via dusrelpdf_flutter_bundled_libraries in CMakeLists.txt) directly
// through dart:ffi. This file exists only because the Flutter tooling
// requires a plugin target for every platform entry in pubspec.yaml.

#include "include/dusrelpdf_flutter/dusrelpdf_flutter_plugin.h"

#include <flutter_linux/flutter_linux.h>

void dusrelpdf_flutter_plugin_register_with_registrar(
    FlPluginRegistrar* registrar) {
  // Nothing to register.
  (void)registrar;
}
