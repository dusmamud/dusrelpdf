# The Flutter tooling requires that developers have CMake 3.10 or later
# installed. You should not increase this version, as doing so will cause
# the plugin to fail to compile for some customers of the plugin.
cmake_minimum_required(VERSION 3.10)

# Project-level configuration.
set(PROJECT_NAME "dusrelpdf_flutter")
project(${PROJECT_NAME} LANGUAGES CXX)

# This value is used when generating builds using this plugin, so it must
# not be changed.
set(PLUGIN_NAME "dusrelpdf_flutter_plugin")

# Any new source files that you add to the plugin should be added here.
add_library(${PLUGIN_NAME} SHARED
  "dusrelpdf_flutter_plugin.cc"
)

# Define the plugin library target.
apply_standard_settings(${PLUGIN_NAME})

set_target_properties(${PLUGIN_NAME} PROPERTIES
  CXX_VISIBILITY_PRESET hidden)
target_compile_definitions(${PLUGIN_NAME} PRIVATE FLUTTER_PLUGIN_IMPL)
target_include_directories(${PLUGIN_NAME} INTERFACE
  "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_link_libraries(${PLUGIN_NAME} PRIVATE flutter)
target_link_libraries(${PLUGIN_NAME} PRIVATE PkgConfig::GTK)

# ---------------------------------------------------------------------------
# Native library sourcing: libdusrelpdf.so
#
# This plugin is an *ffi plugin* — the Dart side loads the library by its
# bare name, and the Flutter tool bundles whatever path is listed in
# `dusrelpdf_flutter_bundled_libraries` below. Resolution order:
#
#   1. Explicit path: -DDUSRELPDFFI_LIB=/path/to/libdusrelpdf.so, or the
#      DUSRELPDFFI_LIB environment variable.
#   2. Repo layout (works today inside this repo): the cdylib built by
#      `cargo build -p dusrelpdf-ffi --release` at the repo root.
#   3. Plugin-local copy: linux/third_party/libdusrelpdf.so.
#   4. FUTURE GitHub Release archive (does NOT exist yet): a pinned
#      URL is downloaded and its SHA-256 verified — see the
#      PLACEHOLDER below. The two-phase step after the first release:
#        a. Publish dusrelpdf-linux-<arch>.tar.gz in the v0.1.0
#           GitHub Release (archive containing libdusrelpdf.so).
#        b. Run: sha256sum dusrelpdf-linux-<arch>.tar.gz
#        c. Replace DUSRELPDFFI_LINUX_SHA256 below with that output.
#      Until then the build fails loudly with instructions instead of
#      downloading anything unverified. Do NOT invent a checksum.
# ---------------------------------------------------------------------------

if(DEFINED ENV{DUSRELPDFFI_LIB} AND NOT "$ENV{DUSRELPDFFI_LIB}" STREQUAL "")
  set(DUSRELPDFFI_LIB_PATH "$ENV{DUSRELPDFFI_LIB}")
elseif(DEFINED DUSRELPDFFI_LIB)
  set(DUSRELPDFFI_LIB_PATH "${DUSRELPDFFI_LIB}")
elseif(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release/libdusrelpdf.so")
  set(DUSRELPDFFI_LIB_PATH "${CMAKE_CURRENT_SOURCE_DIR}/../../../target/release/libdusrelpdf.so")
elseif(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/third_party/libdusrelpdf.so")
  set(DUSRELPDFFI_LIB_PATH "${CMAKE_CURRENT_SOURCE_DIR}/third_party/libdusrelpdf.so")
else()
  # No local library — download the prebuilt release archive.
  if("${CMAKE_SYSTEM_PROCESSOR}" MATCHES "^(aarch64|arm64|ARM64)$")
    set(DUSRELPDFFI_LINUX_ARCH "aarch64")
  else()
    set(DUSRELPDFFI_LINUX_ARCH "x86_64")
  endif()
  # PINNED URL (FUTURE — the v0.1.0 GitHub Release does not exist yet).
  set(DUSRELPDFFI_LINUX_URL
    "https://github.com/dusmamud/dusrelpdf/releases/download/v0.1.0/dusrelpdf-linux-${DUSRELPDFFI_LINUX_ARCH}.tar.gz")
  # >>> PLACEHOLDER — NOT a real checksum. <<<
  set(DUSRELPDFFI_LINUX_SHA256
    "REPLACE_WITH_SHA256_OF_DUSRELPDFFI_LINUX_TAR_GZ_AFTER_FIRST_RELEASE")
  if("${DUSRELPDFFI_LINUX_SHA256}" STREQUAL "REPLACE_WITH_SHA256_OF_DUSRELPDFFI_LINUX_TAR_GZ_AFTER_FIRST_RELEASE")
    message(FATAL_ERROR
      "dusrelpdf_flutter: no local libdusrelpdf.so found and the "
      "prebuilt Linux archive is not released yet. Either build it "
      "locally (`cargo build -p dusrelpdf-ffi --release` from the repo "
      "root), point -DDUSRELPDFFI_LIB at a built .so, or publish "
      "dusrelpdf-linux-${DUSRELPDFFI_LINUX_ARCH}.tar.gz in the v0.1.0 "
      "GitHub Release and set DUSRELPDFFI_LINUX_SHA256 in "
      "linux/CMakeLists.txt to its SHA-256.")
  endif()
  set(_dusrel_archive "${CMAKE_CURRENT_BINARY_DIR}/dusrelpdf-linux.tar.gz")
  file(DOWNLOAD "${DUSRELPDFFI_LINUX_URL}" "${_dusrel_archive}"
    EXPECTED_HASH SHA256=${DUSRELPDFFI_LINUX_SHA256}
    STATUS _dusrel_status
    SHOW_PROGRESS)
  list(GET _dusrel_status 0 _dusrel_code)
  if(NOT "${_dusrel_code}" STREQUAL "0")
    message(FATAL_ERROR "dusrelpdf_flutter: failed to download ${DUSRELPDFFI_LINUX_URL}")
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
  set(DUSRELPDFFI_LIB_PATH "${_dusrel_extract_dir}/libdusrelpdf.so")
endif()

# List of absolute paths to libraries that should be bundled with the plugin.
# The Flutter tool copies these next to the app binary, where the Dart
# side finds libdusrelpdf.so by its bare name.
set(dusrelpdf_flutter_bundled_libraries
  "${DUSRELPDFFI_LIB_PATH}"
  PARENT_SCOPE
)
