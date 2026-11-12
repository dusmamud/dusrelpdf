/// Dart/Flutter bindings for dusrelpdf.
///
/// Thin dart:ffi wrappers over the dusrelpdf Rust core (via the C ABI
/// in `crates/dusrelpdf-ffi`). All PDF logic lives in Rust; this file
/// only converts Dart values to native buffers / JSON options and back.
library;

export 'src/dusrelpdf.dart';
