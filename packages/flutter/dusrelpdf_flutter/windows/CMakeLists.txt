# The Flutter tooling requires that developers have a version of Visual
# Studio with a version number exactly as listed here and a version of
# CMake that is greater than or equal to 3.14. You should not increase the
# version number of CMake, as doing so will cause the plugin to fail to
# compile for some customers of the plugin.
cmake_minimum_required(VERSION 3.14)

# Project-level configuration.
set(PROJECT_NAME "dusrelpdf_flutter")
project(${PROJECT_NAME} LANGUAGES CXX)

# This value is used when generating builds using this plugin, so it must
# not be changed.
set(PLUGIN_NAME "dusrelpdf_flutter_plugin")

# Any new source files that you add to the plugin should be added here.
add_library(${PLUGIN_NAME} SHARED
  "include/dusrelpdf_flutter/dusrelpdf_flutter_plugin.h"
  "include/dusrelpdf_flutter/dusrelpdf_flutter_plugin_c_api.h"
  "dusrelpdf_flutter_plugin.cpp"
)

# Define the plugin library target.
apply_standard_settings(${PLUGIN_NAME})

set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden)
target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)
target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_link_libraries(${PLUGIN_NAME} PRIVATE flutter_wrapper_plugin)

# ---------------------------------------------------------------------------
# Native library sourcing: dusrelpdf.dll
#
# This plugin is an *ffi plugin* — the Dart side loads the library by its
# bare name, and the Flutter tool bundles whatever path is listed in
# `dusrelpdf_flutter_bundled_libraries` below. Resolution order:
#
#   1. Explicit path: -DDUSRELPDFFI_LIB=C:/path/to/dusrelpdf.dll, or the
#      DUSRELPDFFI_LIB environment variable.
#   2. Repo layout (works today inside this repo): the cdylib built by
#      `cargo build -p dusrelpdf-ffi --release` at the repo root
#      (target/release/dusrelpdf.dll).
#   3. Plugin-local copy: windows/third_party/dusrelpdf.dll.
#   4. FUTURE GitHub Release archive (does NOT exist yet): a pinned
#      URL is downloaded and its SHA-256 verified — see the
#      PLACEHOLDER below. The two-phase step after the first release:
#        a. Publish dusrelpdf-windows-<arch>.zip in the v0.1.0
#           GitHub Release (archive containing dusrelpdf.dll).
#        b. Run: (Get-FileHash dusrelpdf-windows-<arch>.zip).Hash
#        c. Replace DUSRELPDFFI_WINDOWS_SHA256 below with that output.
#      Until then the build fails loudly with instructions instead of
#      downloading anything unverified. Do NOT invent a checksum.
# ---------------------------------------------------------------------------

if(DEFINED ENV{DUSRELPDFFI_LIB} AND NOT "$ENV{DUSRELPDFFI_LIB}" STREQUAL "")
  set(DUSRELPDFFI_LIB_PATH "$ENV{DUSRELPDFFI_LIB}")
elseif(DEFINED DUSRELPDFFI_LIB)
  set(DUSRELPDFFI_LIB_PATH "${DUSRELPDFFI_LIB}")
elseif(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release/dusrelpdf.dll")
  set(DUSRELPDFFI_LIB_PATH "${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release/dusrelpdf.dll")
elseif(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/third_party/dusrelpdf.dll")
  set(DUSRELPDFFI_LIB_PATH "${CMAKE_CURRENT_SOURCE_DIR}/third_party/dusrelpdf.dll")
else()
  # No local library — download the prebuilt release archive.
  if("${CMAKE_SYSTEM_PROCESSOR}" MATCHES "^(ARM64|arm64)$")
    set(DUSRELPDFFI_WINDOWS_ARCH "arm64")
  else()
    set(DUSRELPDFFI_WINDOWS_ARCH "x64")
  endif()
  # PINNED URL (FUTURE — the v0.1.0 GitHub Release does not exist yet).
  set(DUSRELPDFFI_WINDOWS_URL
    "https://github.com/dusmamud/dusrelpdf/releases/download/v0.1.0/dusrelpdf-windows-${DUSRELPDFFI_WINDOWS_ARCH}.zip")
  # >>> PLACEHOLDER — NOT a real checksum. <<<
  set(DUSRELPDFFI_WINDOWS_SHA256
    "REPLACE_WITH_SHA256_OF_DUSRELPDFFI_WINDOWS_ZIP_AFTER_FIRST_RELEASE")
  if("${DUSRELPDFFI_WINDOWS_SHA256}" STREQUAL "REPLACE_WITH_SHA256_OF_DUSRELPDFFI_WINDOWS_ZIP_AFTER_FIRST_RELEASE")
    message(FATAL_ERROR
      "dusrelpdf_flutter: no local dusrelpdf.dll found and the "
      "prebuilt Windows archive is not released yet. Either build it "
      "locally (`cargo build -p dusrelpdf-ffi --release` from the repo "
      "root), point -DDUSRELPDFFI_LIB at a built .dll, or publish "
      "dusrelpdf-windows-${DUSRELPDFFI_WINDOWS_ARCH}.zip in the v0.1.0 "
      "GitHub Release and set DUSRELPDFFI_WINDOWS_SHA256 in "
      "windows/CMakeLists.txt to its SHA-256.")
  endif()
  set(_dusrel_archive "${CMAKE_CURRENT_BINARY_DIR}/dusrelpdf-windows.zip")
  file(DOWNLOAD "${DUSRELPDFFI_WINDOWS_URL}" "${_dusrel_archive}"
    EXPECTED_HASH SHA256=${DUSRELPDFFI_WINDOWS_SHA256}
    STATUS _dusrel_status
    SHOW_PROGRESS)
  list(GET _dusrel_status 0 _dusrel_code)
  if(NOT "${_dusrel_code}" STREQUAL "0")
    message(FATAL_ERROR "dusrelpdf_flutter: failed to download ${DUSRELPDFFI_WINDOWS_URL}")
  endif()
  set(_dusrel_extract_dir "${CMAKE_CURRENT_BINARY_DIR}/dusrelpdf-ffi")
  file(MAKE_DIRECTORY "${_dusrel_extract_dir}")
  execute_process(
    COMMAND "${CMAKE_COMMAND}" -E tar xzf "${_dusrel_archive}"
    WORKING_DIRECTORY "${_dusrel_extract_dir}"
    RESULT_VARIABLE _dusrel_extract_result)
  if(NOT "${_dusrel_extract_result}" STREQUAL "0")
    message(FATAL_ERROR "dusrelpdf_flutter: failed to extract ${_dusrel_archive}")
  endif()
  set(DUSRELPDFFI_LIB_PATH "${_dusrel_extract_dir}/dusrelpdf.dll")
endif()

# List of absolute paths to libraries that should be bundled with the plugin.
# The Flutter tool copies these next to the app .exe, where the Dart side
# finds dusrelpdf.dll by its bare name.
set(dusrelpdf_flutter_bundled_libraries
  "${DUSRELPDFFI_LIB_PATH}"
  PARENT_SCOPE
)
