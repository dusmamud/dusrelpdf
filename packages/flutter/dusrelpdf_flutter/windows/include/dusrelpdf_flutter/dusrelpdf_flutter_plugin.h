#ifndef FLUTTER_PLUGIN_DUSRELPDFFLUTTER_PLUGIN_H_
#define FLUTTER_PLUGIN_DUSRELPDFFLUTTER_PLUGIN_H_

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>

#include <memory>

namespace dusrelpdf_flutter {

// No-op registrant: this is an ffi plugin, so there is no MethodChannel
// and no platform-side API. The Dart side loads dusrelpdf.dll (bundled
// via dusrelpdf_flutter_bundled_libraries in CMakeLists.txt) directly
// through dart:ffi.
class DusrelpdfFlutterPlugin : public flutter::Plugin {
 public:
  static void RegisterWithRegistrar(
      flutter::PluginRegistrarWindows* registrar);

  DusrelpdfFlutterPlugin();

  virtual ~DusrelpdfFlutterPlugin();
};

}  // namespace dusrelpdf_flutter

#endif  // FLUTTER_PLUGIN_DUSRELPDFFLUTTER_PLUGIN_H_
