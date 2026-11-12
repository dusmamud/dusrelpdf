import 'dart:convert';
import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

/// A dusrelpdf failure: a stable machine-readable [code] plus
/// human-readable reasons in English ([messageEn]) and Hindi
/// ([messageHi]). Thrown by every operation on failure.
class DusrelException implements Exception {
  const DusrelException({
    required this.code,
    required this.messageEn,
    required this.messageHi,
  });

  factory DusrelException.fromJson(Map<String, dynamic> json) {
    return DusrelException(
      code: json['code'] as String? ?? 'OPERATION_FAILED',
      messageEn: json['message_en'] as String? ?? '',
      messageHi: json['message_hi'] as String? ?? '',
    );
  }

  /// Stable code, e.g. `INVALID_RANGE`, `PAGE_OUT_OF_RANGE`.
  final String code;

  /// Human-readable reason in English.
  final String messageEn;

  /// Human-readable reason in Hindi.
  final String messageHi;

  @override
  String toString() => '$code: $messageEn';
}

/// Summary of a PDF document, from [Dusrelpdf.getPdfInfo].
class PdfInfo {
  const PdfInfo({
    required this.pageCount,
    required this.version,
    required this.encrypted,
    required this.title,
    required this.pageSizes,
  });

  factory PdfInfo.fromJson(Map<String, dynamic> json) {
    final sizes = (json['page_sizes'] as List<dynamic>? ?? const [])
        .map((s) => [(s[0] as num).toDouble(), (s[1] as num).toDouble()])
        .toList();
    return PdfInfo(
      pageCount: json['page_count'] as int,
      version: json['version'] as String,
      encrypted: json['encrypted'] as bool,
      title: json['title'] as String?,
      pageSizes: sizes,
    );
  }

  /// Number of pages.
  final int pageCount;

  /// PDF version string, e.g. `"1.7"`.
  final String version;

  /// True when the trailer has an `/Encrypt` entry.
  final bool encrypted;

  /// Document title from the Info dict, if present.
  final String? title;

  /// Page sizes in points (width, height), in page order.
  final List<List<double>> pageSizes;
}

/// One entry of a [Dusrelpdf.bulkExtract] result. On success [pdf]
/// holds the output bytes; on failure [pdf] is null and [errorCode] /
/// [error] are set.
class BulkItem {
  const BulkItem({
    required this.index,
    required this.pdf,
    required this.errorCode,
    required this.error,
  });

  factory BulkItem.fromJson(Map<String, dynamic> json) {
    final encoded = json['pdf'] as String?;
    return BulkItem(
      index: json['index'] as int,
      pdf: encoded == null ? null : base64Decode(encoded),
      errorCode: json['error_code'] as String?,
      error: json['error'] as String?,
    );
  }

  /// Input index (0-based).
  final int index;

  /// Output bytes on success; null on failure.
  final Uint8List? pdf;

  /// Error code on failure.
  final String? errorCode;

  /// Human-readable error (English) on failure.
  final String? error;
}

// ---------------------------------------------------------------------------
// Native structs (mirror crates/dusrelpdf-ffi/dusrelpdf.h)
// ---------------------------------------------------------------------------

final class _DusrelBytes extends Struct {
  external Pointer<Uint8> ptr;

  @UintPtr()
  external int len;
}

final class _DusrelResult extends Struct {
  @Uint8()
  external int ok;

  external Pointer<Uint8> data;

  @UintPtr()
  external int dataLen;

  external Pointer<Utf8> errorJson;
}

final class _DusrelResultList extends Struct {
  @Uint8()
  external int ok;

  external Pointer<_DusrelBytes> items;

  @UintPtr()
  external int count;

  external Pointer<Utf8> errorJson;
}

// ---------------------------------------------------------------------------
// Dusrelpdf
// ---------------------------------------------------------------------------

/// Dart/Flutter bindings for dusrelpdf — offline PDF toolkit.
///
/// Every operation takes bytes in and returns bytes out. Options are
/// partial JSON maps with the same snake_case field names as the Rust
/// core options (see `crates/dusrelpdf/src/options.rs`); missing keys
/// keep the Rust defaults, so `null` options means "all defaults".
///
/// ```dart
/// final pdf = Dusrelpdf();
/// final merged = pdf.merge([a, b]);
/// final info = pdf.getPdfInfo(merged);
/// print(info.pageCount);
/// ```
///
/// The native library is loaded from [libraryPath], or resolved
/// automatically: the `DUSRELPDFFI_LIB` environment variable, then the
/// repo-layout default
/// `../../../target/release/libdusrelpdf.{so,dylib,dll}` relative to
/// the current working directory (what `dart test` uses), then the
/// bare library name on the system search path. Build it first with
/// `cargo build -p dusrelpdf-ffi --release` (see README.md).
class Dusrelpdf {
  Dusrelpdf({String? libraryPath})
      : _lib = DynamicLibrary.open(libraryPath ?? _resolveLibraryPath()) {
    _version =
        _lib.lookupFunction<Pointer<Utf8> Function(), Pointer<Utf8> Function()>(
            'dusrelpdf_version');
    _freeResult = _lib.lookupFunction<Void Function(Pointer<_DusrelResult>),
        void Function(Pointer<_DusrelResult>)>('dusrelpdf_free_result');
    _freeResultList = _lib.lookupFunction<
        Void Function(Pointer<_DusrelResultList>),
        void Function(
            Pointer<_DusrelResultList>)>('dusrelpdf_free_result_list');
    _merge = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<_DusrelBytes>, IntPtr, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(
            Pointer<_DusrelBytes>, int, Pointer<Utf8>)>('dusrelpdf_merge');
    _split = _lib.lookupFunction<
        Pointer<_DusrelResultList> Function(
            Pointer<Uint8>, IntPtr, Pointer<Utf8>, Pointer<Utf8>),
        Pointer<_DusrelResultList> Function(Pointer<Uint8>, int, Pointer<Utf8>,
            Pointer<Utf8>)>('dusrelpdf_split');
    _splitRanges = _lib.lookupFunction<
        Pointer<_DusrelResultList> Function(
            Pointer<Uint8>, IntPtr, Pointer<Utf8>, Pointer<Utf8>),
        Pointer<_DusrelResultList> Function(Pointer<Uint8>, int, Pointer<Utf8>,
            Pointer<Utf8>)>('dusrelpdf_split_ranges');
    _splitEachPage = _lib.lookupFunction<
        Pointer<_DusrelResultList> Function(
            Pointer<Uint8>, IntPtr, Pointer<Utf8>),
        Pointer<_DusrelResultList> Function(
            Pointer<Uint8>, int, Pointer<Utf8>)>('dusrelpdf_split_each_page');
    _extractPages = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, IntPtr, Pointer<Utf8>, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Utf8>,
            Pointer<Utf8>)>('dusrelpdf_extract_pages');
    _extractPagesRange = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, IntPtr, Pointer<Utf8>, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Utf8>,
            Pointer<Utf8>)>('dusrelpdf_extract_pages_range');
    _removePages = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, IntPtr, Pointer<Utf8>, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Utf8>,
            Pointer<Utf8>)>('dusrelpdf_remove_pages');
    _reorderPages = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, IntPtr, Pointer<Utf8>, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Utf8>,
            Pointer<Utf8>)>('dusrelpdf_reorder_pages');
    _rotatePages = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(Pointer<Uint8>, IntPtr, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, int, Pointer<Utf8>)>('dusrelpdf_rotate_pages');
    _getPdfInfo = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(Pointer<Uint8>, IntPtr, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, int, Pointer<Utf8>)>('dusrelpdf_get_pdf_info');
    _imagesToPdf = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<_DusrelBytes>, IntPtr, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(Pointer<_DusrelBytes>, int,
            Pointer<Utf8>)>('dusrelpdf_images_to_pdf');
    _addWatermark = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, IntPtr, Pointer<Uint8>, IntPtr, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Uint8>,
            int, Pointer<Utf8>)>('dusrelpdf_add_watermark');
    _addPageNumbers = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(Pointer<Uint8>, IntPtr, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, int, Pointer<Utf8>)>('dusrelpdf_add_page_numbers');
    _placeSignature = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, IntPtr, Pointer<Uint8>, IntPtr, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Uint8>,
            int, Pointer<Utf8>)>('dusrelpdf_place_signature');
    _bulkExtract = _lib.lookupFunction<
        Pointer<_DusrelResult> Function(
            Pointer<Uint8>, IntPtr, Pointer<Utf8>, Pointer<Utf8>),
        Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Utf8>,
            Pointer<Utf8>)>('dusrelpdf_bulk_extract');
  }

  static String _resolveLibraryPath() {
    final env = Platform.environment['DUSRELPDFFI_LIB'];
    if (env != null && env.isNotEmpty) return env;
    final name = Platform.isMacOS
        ? 'libdusrelpdf.dylib'
        : Platform.isWindows
            ? 'dusrelpdf.dll'
            : 'libdusrelpdf.so';
    // Repo-layout candidates, first existing file wins. `dart test`
    // runs with the package directory as the working directory.
    for (final candidate in [
      '../../../target/release/$name',
      '../../target/release/$name',
      '../target/release/$name',
    ]) {
      if (File(candidate).existsSync()) return candidate;
    }
    // Fall back to the system library search path.
    return name;
  }

  final DynamicLibrary _lib;

  late final Pointer<Utf8> Function() _version;
  late final void Function(Pointer<_DusrelResult>) _freeResult;
  late final void Function(Pointer<_DusrelResultList>) _freeResultList;
  late final Pointer<_DusrelResult> Function(
      Pointer<_DusrelBytes>, int, Pointer<Utf8>) _merge;
  late final Pointer<_DusrelResultList> Function(
      Pointer<Uint8>, int, Pointer<Utf8>, Pointer<Utf8>) _split;
  late final Pointer<_DusrelResultList> Function(
      Pointer<Uint8>, int, Pointer<Utf8>, Pointer<Utf8>) _splitRanges;
  late final Pointer<_DusrelResultList> Function(
      Pointer<Uint8>, int, Pointer<Utf8>) _splitEachPage;
  late final Pointer<_DusrelResult> Function(
      Pointer<Uint8>, int, Pointer<Utf8>, Pointer<Utf8>) _extractPages;
  late final Pointer<_DusrelResult> Function(
      Pointer<Uint8>, int, Pointer<Utf8>, Pointer<Utf8>) _extractPagesRange;
  late final Pointer<_DusrelResult> Function(
      Pointer<Uint8>, int, Pointer<Utf8>, Pointer<Utf8>) _removePages;
  late final Pointer<_DusrelResult> Function(
      Pointer<Uint8>, int, Pointer<Utf8>, Pointer<Utf8>) _reorderPages;
  late final Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Utf8>)
      _rotatePages;
  late final Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Utf8>)
      _getPdfInfo;
  late final Pointer<_DusrelResult> Function(
      Pointer<_DusrelBytes>, int, Pointer<Utf8>) _imagesToPdf;
  late final Pointer<_DusrelResult> Function(
      Pointer<Uint8>, int, Pointer<Uint8>, int, Pointer<Utf8>) _addWatermark;
  late final Pointer<_DusrelResult> Function(Pointer<Uint8>, int, Pointer<Utf8>)
      _addPageNumbers;
  late final Pointer<_DusrelResult> Function(
      Pointer<Uint8>, int, Pointer<Uint8>, int, Pointer<Utf8>) _placeSignature;
  late final Pointer<_DusrelResult> Function(
      Pointer<Uint8>, int, Pointer<Utf8>, Pointer<Utf8>) _bulkExtract;

  /// Library version, e.g. `"0.1.0"`.
  String get version => _version().toDartString();

  // -- native helpers -------------------------------------------------------

  /// Copy [bytes] into native memory. Empty input yields [nullptr].
  /// Every non-null pointer returned here must be freed by the caller.
  Pointer<Uint8> _toNative(Uint8List bytes, List<Pointer> allocs) {
    if (bytes.isEmpty) return nullptr;
    final ptr = calloc<Uint8>(bytes.length);
    ptr.asTypedList(bytes.length).setAll(0, bytes);
    allocs.add(ptr);
    return ptr;
  }

  /// Copy a list of byte buffers into a native `_DusrelBytes` array.
  /// Empty input yields [nullptr]. The array pointer (when non-null)
  /// and every buffer pointer are tracked in [allocs] for freeing.
  Pointer<_DusrelBytes> _toNativeArray(
      List<Uint8List> inputs, List<Pointer> allocs) {
    if (inputs.isEmpty) return nullptr;
    final arr = calloc<_DusrelBytes>(inputs.length);
    allocs.add(arr);
    for (var i = 0; i < inputs.length; i++) {
      final slot = (arr + i).ref;
      slot.ptr = _toNative(inputs[i], allocs);
      slot.len = inputs[i].length;
    }
    return arr;
  }

  /// JSON-encode [options] into a native UTF-8 string, or [nullptr]
  /// for null options (all Rust defaults). Tracked in [allocs].
  Pointer<Utf8> _optionsPtr(
      Map<String, dynamic>? options, List<Pointer> allocs) {
    if (options == null) return nullptr;
    final ptr = jsonEncode(options).toNativeUtf8();
    allocs.add(ptr);
    return ptr;
  }

  /// JSON-encode a page list, e.g. `[1, 3]`.
  Pointer<Utf8> _pagesPtr(List<int> pages, List<Pointer> allocs) {
    final ptr = jsonEncode(pages).toNativeUtf8();
    allocs.add(ptr);
    return ptr;
  }

  Never _throwFromErrorJson(Pointer<Utf8> errorJson) {
    final decoded =
        jsonDecode(errorJson.toDartString()) as Map<String, dynamic>;
    throw DusrelException.fromJson(decoded);
  }

  /// Read a single-output result: copy the bytes, then free the
  /// native result. Throws [DusrelException] on `ok == 0`.
  Uint8List _readResult(Pointer<_DusrelResult> result) {
    try {
      final ref = result.ref;
      if (ref.ok == 0) _throwFromErrorJson(ref.errorJson);
      if (ref.data == nullptr || ref.dataLen == 0) return Uint8List(0);
      return Uint8List.fromList(ref.data.asTypedList(ref.dataLen));
    } finally {
      _freeResult(result);
    }
  }

  /// Read a multi-output result: copy every item, then free the
  /// native result. Throws [DusrelException] on `ok == 0`.
  List<Uint8List> _readResultList(Pointer<_DusrelResultList> result) {
    try {
      final ref = result.ref;
      if (ref.ok == 0) _throwFromErrorJson(ref.errorJson);
      final out = <Uint8List>[];
      for (var i = 0; i < ref.count; i++) {
        final item = (ref.items + i).ref;
        if (item.ptr == nullptr || item.len == 0) {
          out.add(Uint8List(0));
        } else {
          out.add(Uint8List.fromList(item.ptr.asTypedList(item.len)));
        }
      }
      return out;
    } finally {
      _freeResultList(result);
    }
  }

  void _freeAll(List<Pointer> allocs) {
    // UTF-8 option strings come from malloc (toNativeUtf8); byte
    // buffers and arrays come from calloc. Both free through the
    // same underlying free, so one loop covers both.
    for (final p in allocs) {
      malloc.free(p);
    }
  }

  // -- operations -----------------------------------------------------------

  /// Merge several PDFs into one, in input order.
  ///
  /// [options] is a partial `MergeOptions` map, e.g.
  /// `{'preserve_metadata': true, 'add_bookmarks': true}`.
  Uint8List merge(List<Uint8List> pdfs, {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final arr = _toNativeArray(pdfs, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(_merge(arr, pdfs.length, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Split one PDF into one output per range. [ranges] is a list of
  /// page-number lists (1-based), e.g. `[[1, 2], [3, 4, 5]]`.
  List<Uint8List> split(Uint8List pdf, List<List<int>> ranges,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final r = jsonEncode(ranges).toNativeUtf8();
      allocs.add(r);
      final opts = _optionsPtr(options, allocs);
      return _readResultList(_split(p, pdf.length, r, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Split one PDF from a range string such as `"1-3,4-5"` — one
  /// output per comma-separated chunk.
  List<Uint8List> splitRanges(Uint8List pdf, String ranges,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final r = ranges.toNativeUtf8();
      allocs.add(r);
      final opts = _optionsPtr(options, allocs);
      return _readResultList(_splitRanges(p, pdf.length, r, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Split one PDF into one output per page.
  List<Uint8List> splitEachPage(Uint8List pdf,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResultList(_splitEachPage(p, pdf.length, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Extract selected pages into a new PDF, in the order given.
  Uint8List extractPages(Uint8List pdf, List<int> pages,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final pg = _pagesPtr(pages, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(_extractPages(p, pdf.length, pg, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Extract pages from a range string such as `"1-3,5"`.
  Uint8List extractPagesRange(Uint8List pdf, String range,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final r = range.toNativeUtf8();
      allocs.add(r);
      final opts = _optionsPtr(options, allocs);
      return _readResult(_extractPagesRange(p, pdf.length, r, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Delete pages (1-based numbers). An empty list returns the input
  /// unchanged.
  Uint8List removePages(Uint8List pdf, List<int> pages,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final pg = _pagesPtr(pages, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(_removePages(p, pdf.length, pg, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Change the page order. [order] must list every page exactly
  /// once, e.g. `[3, 1, 2]`.
  Uint8List reorderPages(Uint8List pdf, List<int> order,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final o = _pagesPtr(order, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(_reorderPages(p, pdf.length, o, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Rotate pages by a multiple of 90 degrees. The angle (and the
  /// optional `pages` list) come from [options], e.g.
  /// `{'angle': 90}` or `{'angle': -90, 'pages': [1]}`. Null options
  /// rotate every page by 90°.
  Uint8List rotate(Uint8List pdf, {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(_rotatePages(p, pdf.length, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Read a PDF's summary without modifying it.
  PdfInfo getPdfInfo(Uint8List pdf, {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final opts = _optionsPtr(options, allocs);
      final data = _readResult(_getPdfInfo(p, pdf.length, opts));
      return PdfInfo.fromJson(
          jsonDecode(utf8.decode(data)) as Map<String, dynamic>);
    } finally {
      _freeAll(allocs);
    }
  }

  /// Build a PDF with one page per image, in input order. Images are
  /// PNG/JPEG bytes. [options] is a partial `ImagesToPdfOptions`
  /// map, e.g. `{'page_size': 'a4', 'fit': 'contain'}`.
  Uint8List imagesToPdf(List<Uint8List> images,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final arr = _toNativeArray(images, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(_imagesToPdf(arr, images.length, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Stamp a text or image watermark over the selected pages. Pass
  /// [image] bytes (PNG/JPEG) for an image watermark, or null plus
  /// `{'text': 'DRAFT'}` in [options] for a text watermark.
  Uint8List addWatermark(Uint8List pdf,
      {Uint8List? image, Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final img = image == null ? nullptr : _toNative(image, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(
          _addWatermark(p, pdf.length, img, image?.length ?? 0, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Stamp page numbers over the selected pages. [options] is a
  /// partial `PageNumberOptions` map, e.g.
  /// `{'format': 'Page {n} of {total}', 'position': 'bottom_center'}`.
  Uint8List addPageNumbers(Uint8List pdf, {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(_addPageNumbers(p, pdf.length, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Place a handwritten signature IMAGE on one page (image placement
  /// only — not a cryptographic digital signature). [image] is the
  /// signature image bytes; a transparent PNG keeps its transparency.
  /// [options] is a partial `SignatureOptions` map, e.g.
  /// `{'page': 1, 'x': 36.0, 'y': 36.0, 'width': 144.0}`.
  Uint8List placeSignature(Uint8List pdf, Uint8List image,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final img = _toNative(image, allocs);
      final opts = _optionsPtr(options, allocs);
      return _readResult(
          _placeSignature(p, pdf.length, img, image.length, opts));
    } finally {
      _freeAll(allocs);
    }
  }

  /// Extract many selections from one PDF in a single call. One bad
  /// selection fails only its own item, never the whole batch.
  List<BulkItem> bulkExtract(Uint8List pdf, List<List<int>> selections,
      {Map<String, dynamic>? options}) {
    final allocs = <Pointer>[];
    try {
      final p = _toNative(pdf, allocs);
      final s = jsonEncode(selections).toNativeUtf8();
      allocs.add(s);
      final opts = _optionsPtr(options, allocs);
      final data = _readResult(_bulkExtract(p, pdf.length, s, opts));
      final items = jsonDecode(utf8.decode(data)) as List<dynamic>;
      return items
          .map((e) => BulkItem.fromJson(e as Map<String, dynamic>))
          .toList();
    } finally {
      _freeAll(allocs);
    }
  }
}
