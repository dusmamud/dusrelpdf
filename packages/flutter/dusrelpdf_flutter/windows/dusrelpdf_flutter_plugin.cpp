// No-op registrant: this is an ffi plugin, so there is no MethodChannel
// and no platform-side API. The Dart side loads dusrelpdf.dll (bundled
// via dusrelpdf_flutter_bundled_libraries in CMakeLists.txt) directly
// through dart:ffi. This file exists only because the Flutter tooling
// requires a plugin target for every platform entry in pubspec.yaml.

#include "include/dusrelpdf_flutter/dusrelpdf_flutter_plugin.h"

#include <flutter/method_channel.h>
#include <flutter/plugin_registrar_windows.h>

#include <memory>

namespace dusrelpdf_flutter {

void DusrelpdfFlutterPlugin::RegisterWithRegistrar(
    flutter::PluginRegistrarWindows* registrar) {
  // Nothing to register: no MethodChannel, no platform API.
  auto plugin = std::make_unique<DusrelpdfFlutterPlugin>();
  registrar->AddPlugin(std::move(plugin));
}

DusrelpdfFlutterPlugin::DusrelpdfFlutterPlugin() {}

DusrelpdfFlutterPlugin::~DusrelpdfFlutterPlugin() {}

}  // namespace dusrelpdf_flutter

void DusrelpdfFlutterPluginRegisterWithRegistrar(
    FlutterDesktopPluginRegistrarRef registrar) {
  dusrelpdf_flutter::DusrelpdfFlutterPlugin::RegisterWithRegistrar(
      flutter::PluginRegistrarManager::GetInstance()
          ->GetRegistrar<flutter::PluginRegistrarWindows>(registrar));
}
