#
# dusrelpdf_flutter — iOS podspec.
#
# This plugin is an *ffi plugin*: there is no MethodChannel. The only
# native artifact is the Dusrelpdf XCFramework, which the Dart side
# loads from the app bundle (Frameworks/Dusrelpdf.framework/Dusrelpdf).
#
# Where the XCFramework comes from (in order):
#   1. LOCAL BUILD (works today): build the XCFramework from the Rust
#      core and place it at ios/Frameworks/Dusrelpdf.xcframework
#      (next to this podspec, inside a Frameworks/ directory).
#      prepare_command copies it into place and skips the download.
#   2. FUTURE GitHub Release (does NOT exist yet): the first v0.1.0
#      release publishes dusrelpdf.xcframework.zip as a release
#      asset. prepare_command downloads the pinned URL below and
#      verifies its SHA-256.
#
# >>> PLACEHOLDER: the SHA-256 below is NOT a real checksum. <<<
# The two-phase step after the first release is:
#   a. Publish dusrelpdf.xcframework.zip in the v0.1.0 GitHub Release.
#   b. Run: shasum -a 256 dusrelpdf.xcframework.zip
#   c. Replace DUSRELPDFFI_XCFRAMEWORK_SHA256 below with that output.
# Until then, `pod install` fails loudly with instructions instead
# of downloading anything unverified. Do NOT invent a checksum.
#
Pod::Spec.new do |s|
  s.name             = 'dusrelpdf_flutter'
  s.version          = '0.1.0'
  s.summary          = 'Flutter plugin wiring for dusrelpdf (offline PDF toolkit).'
  s.description      = <<-DESC
    Bundles the dusrelpdf native library (Dusrelpdf.xcframework) into
    a Flutter iOS app so the pure-Dart dusrelpdf package can call it
    via dart:ffi. No MethodChannel; all PDF logic lives in Rust.
  DESC
  s.homepage         = 'https://github.com/dusmamud/dusrelpdf'
  s.license          = { :file => '../LICENSE' }
  s.author           = { 'dusmamud' => 'https://github.com/dusmamud' }
  s.source           = { :path => '.' }
  s.source_files     = 'Classes/**/*'
  s.dependency 'Flutter'
  s.platform         = :ios, '12.0'
  s.swift_version    = '5.0'

  # Pinned release asset (FUTURE — the v0.1.0 GitHub Release does not
  # exist yet; see the header comment and README.md).
  # NOTE: single-quoted heredoc (<<-'SCRIPT') so Ruby does NOT
  # interpolate the shell's $SHA256 / $URL variables.
  s.prepare_command = <<-'SCRIPT'
    set -e
    if [ -d "Frameworks/Dusrelpdf.xcframework" ]; then
      echo "dusrelpdf_flutter: using local Frameworks/Dusrelpdf.xcframework"
      rm -rf "Dusrelpdf.xcframework"
      cp -R "Frameworks/Dusrelpdf.xcframework" "Dusrelpdf.xcframework"
      exit 0
    fi
    if [ -d "Dusrelpdf.xcframework" ]; then
      echo "dusrelpdf_flutter: Dusrelpdf.xcframework already present"
      exit 0
    fi
    SHA256="REPLACE_WITH_SHA256_OF_DUSRELPDFFI_XCFRAMEWORK_ZIP_AFTER_FIRST_RELEASE"
    if [ "$SHA256" = "REPLACE_WITH_SHA256_OF_DUSRELPDFFI_XCFRAMEWORK_ZIP_AFTER_FIRST_RELEASE" ]; then
      echo "error: dusrelpdf_flutter: the Dusrelpdf.xcframework release asset does not exist yet." >&2
      echo "error: either place a locally built XCFramework at ios/Frameworks/Dusrelpdf.xcframework," >&2
      echo "error: or publish dusrelpdf.xcframework.zip in the v0.1.0 GitHub Release and set" >&2
      echo "error: DUSRELPDFFI_XCFRAMEWORK_SHA256 in ios/dusrelpdf_flutter.podspec to its SHA-256." >&2
      exit 1
    fi
    URL="https://github.com/dusmamud/dusrelpdf/releases/download/v0.1.0/dusrelpdf.xcframework.zip"
    echo "dusrelpdf_flutter: downloading $URL"
    curl -L -o dusrelpdf.xcframework.zip "$URL"
    echo "$SHA256  dusrelpdf.xcframework.zip" | shasum -a 256 -c -
    unzip -q -o dusrelpdf.xcframework.zip
    rm dusrelpdf.xcframework.zip
  SCRIPT

  s.vendored_frameworks = 'Dusrelpdf.xcframework'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
end
