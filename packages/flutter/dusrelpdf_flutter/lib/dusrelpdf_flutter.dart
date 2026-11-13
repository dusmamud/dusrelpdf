/// Flutter plugin wiring for dusrelpdf.
///
/// This package holds ONLY the native wiring. All PDF logic lives in the
/// pure-Dart `dusrelpdf` package (dart:ffi over the Rust core); this
/// package re-exports it and adds [DusrelpdfFlutter], which resolves the
/// natively bundled library per platform.
///
/// ```dart
/// import 'package:dusrelpdf_flutter/dusrelpdf_flutter.dart';
///
/// final pdf = DusrelpdfFlutter();
/// final merged = pdf.merge([a, b]);
/// ```
library;

export 'package:dusrelpdf/dusrelpdf.dart';
export 'src/dusrelpdf_flutter.dart';
