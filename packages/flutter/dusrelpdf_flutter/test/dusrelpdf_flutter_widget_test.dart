// Widget-level test for the dusrelpdf Flutter plugin.
//
// REQUIRES THE FLUTTER SDK. Run ONLY with:
//
//   flutter test test/dusrelpdf_flutter_widget_test.dart
//
// (from this package directory, with the Rust cdylib built first:
// `cargo build -p dusrelpdf-ffi --release` from the repo root —
// `flutter test` runs on the host, and DusrelpdfFlutter resolves the
// library via the repo-layout candidates, same as `dart test`.)
//
// NOT RUN on this VM: no Flutter SDK is installed here, and
// `dart test` cannot compile this file (it imports package:flutter).
// Do not run plain `dart test` on the whole test/ directory.

import 'package:dusrelpdf_flutter/dusrelpdf_flutter.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

/// Minimal app widget: shows the native library version reported by
/// the plugin, or the error if the library cannot be loaded.
class VersionApp extends StatefulWidget {
  const VersionApp({super.key});

  @override
  State<VersionApp> createState() => _VersionAppState();
}

class _VersionAppState extends State<VersionApp> {
  String _version = 'loading…';

  @override
  void initState() {
    super.initState();
    try {
      _version = DusrelpdfFlutter().version;
    } on Object catch (e) {
      _version = 'error: $e';
    }
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: Center(
          child: Text(_version),
        ),
      ),
    );
  }
}

void main() {
  testWidgets('DusrelpdfFlutter loads and reports version 0.1.0',
      (WidgetTester tester) async {
    await tester.pumpWidget(const VersionApp());

    // The version comes from the real native library via the plugin.
    expect(find.text('0.1.0'), findsOneWidget);
    expect(find.textContaining('error:'), findsNothing);
  });

  testWidgets('plugin re-export is usable from a widget', (tester) async {
    // Constructing through the re-exported Dusrelpdf type works too.
    final pdf = DusrelpdfFlutter();
    expect(pdf, isA<Dusrelpdf>());
    await tester.pumpWidget(const VersionApp());
    expect(find.text('0.1.0'), findsOneWidget);
  });
}
