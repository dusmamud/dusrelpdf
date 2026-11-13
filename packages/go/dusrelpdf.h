/* dusrelpdf — C ABI (v0.1.0)
 *
 * Thin C wrappers over the dusrelpdf Rust core: merge, split, extract,
 * remove, reorder, rotate, images-to-PDF, watermark, page numbers and
 * signature-image placement. Every operation takes bytes in and returns
 * bytes out; errors come back as a JSON object with a stable code plus
 * English and Hindi messages.
 *
 * OWNERSHIP (read this before calling anything):
 * - Inputs are BORROWED: byte buffers, DusrelBytes arrays and JSON
 *   strings are read only for the duration of the call. You keep
 *   owning them; the library never frees them and never holds a
 *   pointer after returning.
 * - Outputs are CALLER-OWNED: every operation returns a heap
 *   allocated DusrelResult* (single output) or DusrelResultList*
 *   (multi-output: split family). Free it EXACTLY ONCE with the
 *   matching free function (dusrelpdf_free_result /
 *   dusrelpdf_free_result_list), which frees the struct, the output
 *   byte buffer(s) and the error string. Do not free the inner
 *   buffers yourself, do not free twice, do not mix the two free
 *   functions. Both free functions accept NULL (no-op).
 * - On success (ok == 1): `data` holds the output bytes, `error_json`
 *   is NULL. get_pdf_info and bulk_extract return UTF-8 JSON in
 *   `data` instead of PDF bytes (see the Rust docs for the shape).
 * - On failure (ok == 0): `data` is NULL and `error_json` is a
 *   NUL-terminated JSON string {"code": "…", "message_en": "…",
 *   "message_hi": "…"}.
 * - Functions NEVER return NULL and NEVER panic across the FFI
 *   boundary. A NULL/invalid pointer or bad options JSON comes back
 *   as an ok == 0 result. The only exception is dusrelpdf_version(),
 *   which returns a static string you must NOT free.
 * - Options are partial JSON objects with the same snake_case field
 *   names as the Rust core options (see crates/dusrelpdf/src/options.rs).
 *   Missing keys keep the Rust defaults; NULL or "" means all defaults.
 *
 * SPDX-License-Identifier: MIT
 */

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stddef.h>
/* See the header comment above for the ownership contract. */

/**
 * Result of an operation that produces a single output.
 */
typedef struct DusrelResult {
  /**
   * 1 on success, 0 on error.
   */
  uint8_t ok;
  /**
   * Output bytes on success; NULL on error. Caller-owned — free
   * with `dusrelpdf_free_result`, never directly.
   */
  unsigned char *data;
  /**
   * Length of `data` in bytes; 0 on error.
   */
  uintptr_t data_len;
  /**
   * NULL on success. On error, a NUL-terminated JSON string
   * `{"code": "…", "message_en": "…", "message_hi": "…"}`.
   * Caller-owned — freed by `dusrelpdf_free_result`.
   */
  char *error_json;
} DusrelResult;

/**
 * One borrowed byte buffer (an input PDF, an image, …).
 */
typedef struct DusrelBytes {
  /**
   * Pointer to the bytes. Borrowed for the call only; may be NULL
   * only when `len` is 0.
   */
  const unsigned char *ptr;
  /**
   * Number of bytes.
   */
  uintptr_t len;
} DusrelBytes;

/**
 * Result of an operation that produces several outputs (split).
 */
typedef struct DusrelResultList {
  /**
   * 1 on success, 0 on error.
   */
  uint8_t ok;
  /**
   * Array of `count` output buffers on success; NULL on error.
   * Each `items[i].ptr` is caller-owned — freed by
   * `dusrelpdf_free_result_list`, never directly.
   */
  struct DusrelBytes *items;
  /**
   * Number of items; 0 on error.
   */
  uintptr_t count;
  /**
   * NULL on success. On error, a NUL-terminated JSON string
   * `{"code": "…", "message_en": "…", "message_hi": "…"}`.
   * Caller-owned — freed by `dusrelpdf_free_result_list`.
   */
  char *error_json;
} DusrelResultList;

/**
 * Library version, e.g. `"0.1.0"`. Static string — do NOT free it.
 */
const char *dusrelpdf_version(void);

/**
 * Free a `DusrelResult` returned by any single-output operation,
 * including its output bytes and error string. Accepts NULL (no-op).
 * Must be called exactly once per result.
 *
 * # Safety
 * `result` must be a pointer previously returned by this library
 * (or NULL), and must not have been freed already.
 */
void dusrelpdf_free_result(struct DusrelResult *result);

/**
 * Free a `DusrelResultList` returned by any multi-output operation,
 * including every output buffer and the error string. Accepts NULL
 * (no-op). Must be called exactly once per result.
 *
 * # Safety
 * `result` must be a pointer previously returned by this library
 * (or NULL), and must not have been freed already.
 */
void dusrelpdf_free_result_list(struct DusrelResultList *result);

/**
 * Merge several PDFs into one, in input order.
 *
 * `inputs` is an array of `count` byte buffers. `options_json` is a
 * partial `MergeOptions` JSON object (NULL/empty = defaults:
 * `preserve_metadata` and `add_bookmarks` both true).
 *
 * # Safety
 * `inputs` must point at `count` readable `DusrelBytes`, each
 * pointing at `len` readable bytes (or NULL when `len` is 0).
 * `options_json` must be NULL or a NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_merge(const struct DusrelBytes *inputs,
                                     uintptr_t count,
                                     const char *options_json);

/**
 * Split one PDF into one output per range. `ranges_json` is a JSON
 * array of page-number arrays, e.g. `"[[1, 2], [3]]"`.
 * `options_json` is a partial `PageSetOptions` JSON object.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `ranges_json` must
 * be a NUL-terminated string. `options_json` must be NULL or a
 * NUL-terminated string.
 */
struct DusrelResultList *dusrelpdf_split(const unsigned char *pdf,
                                         uintptr_t pdf_len,
                                         const char *ranges_json,
                                         const char *options_json);

/**
 * Split one PDF from a range string such as `"1-3,4-5"` — one output
 * per comma-separated chunk. `options_json` is a partial
 * `PageSetOptions` JSON object.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `ranges` and
 * `options_json` must be NULL (options only) or NUL-terminated strings.
 */
struct DusrelResultList *dusrelpdf_split_ranges(const unsigned char *pdf,
                                                uintptr_t pdf_len,
                                                const char *ranges,
                                                const char *options_json);

/**
 * Split one PDF into one output per page.
 * `options_json` is a partial `PageSetOptions` JSON object.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `options_json` must
 * be NULL or a NUL-terminated string.
 */
struct DusrelResultList *dusrelpdf_split_each_page(const unsigned char *pdf,
                                                   uintptr_t pdf_len,
                                                   const char *options_json);

/**
 * Extract selected pages into a new PDF, in the order given.
 * `pages_json` is a JSON array of 1-based page numbers, e.g.
 * `"[1, 3]"`. `options_json` is a partial `PageSetOptions` JSON object.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `pages_json` must be
 * a NUL-terminated string. `options_json` must be NULL or a
 * NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_extract_pages(const unsigned char *pdf,
                                             uintptr_t pdf_len,
                                             const char *pages_json,
                                             const char *options_json);

/**
 * Extract pages from a range string such as `"1-3,5"`.
 * `options_json` is a partial `PageSetOptions` JSON object.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `range` must be a
 * NUL-terminated string. `options_json` must be NULL or a
 * NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_extract_pages_range(const unsigned char *pdf,
                                                   uintptr_t pdf_len,
                                                   const char *range,
                                                   const char *options_json);

/**
 * Delete pages (1-based numbers in `pages_json`, e.g. `"[2]"`).
 * `options_json` is a partial `PageSetOptions` JSON object.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `pages_json` must be
 * a NUL-terminated string. `options_json` must be NULL or a
 * NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_remove_pages(const unsigned char *pdf,
                                            uintptr_t pdf_len,
                                            const char *pages_json,
                                            const char *options_json);

/**
 * Change the page order. `order_json` is a JSON array listing every
 * page exactly once, e.g. `"[3, 1, 2]"`. `options_json` is a partial
 * `PageSetOptions` JSON object.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `order_json` must be
 * a NUL-terminated string. `options_json` must be NULL or a
 * NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_reorder_pages(const unsigned char *pdf,
                                             uintptr_t pdf_len,
                                             const char *order_json,
                                             const char *options_json);

/**
 * Rotate pages by a multiple of 90 degrees. The angle (and the
 * optional `pages` list) come from `options_json`, a partial
 * `RotateOptions` JSON object, e.g. `{"angle": 90}` or
 * `{"angle": -90, "pages": [1]}`. NULL/empty = rotate every page
 * by 90°.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `options_json` must
 * be NULL or a NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_rotate_pages(const unsigned char *pdf,
                                            uintptr_t pdf_len,
                                            const char *options_json);

/**
 * Read a PDF's summary without modifying it. On success `data` is
 * UTF-8 JSON of the `PdfInfo` struct: `{"page_count": 3,
 * "version": "1.7", "encrypted": false, "title": "…",
 * "page_sizes": [[595.0, 842.0], …]}`. `options_json` is a partial
 * `CommonOptions` JSON object (only `password` / `compress` apply).
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `options_json` must
 * be NULL or a NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_get_pdf_info(const unsigned char *pdf,
                                            uintptr_t pdf_len,
                                            const char *options_json);

/**
 * Build a PDF with one page per image, in input order. `images` is
 * an array of `count` image buffers (PNG/JPEG). `options_json` is a
 * partial `ImagesToPdfOptions` JSON object, e.g.
 * `{"page_size": "a4", "fit": "contain"}`.
 *
 * # Safety
 * `images` must point at `count` readable `DusrelBytes`, each
 * pointing at `len` readable bytes (or NULL when `len` is 0).
 * `options_json` must be NULL or a NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_images_to_pdf(const struct DusrelBytes *images,
                                             uintptr_t count,
                                             const char *options_json);

/**
 * Stamp a text or image watermark over the selected pages.
 * `options_json` is a partial `WatermarkOptions` JSON object, e.g.
 * `{"text": "DRAFT", "opacity": 0.3, "rotation": 45}`. When `image`
 * is non-NULL it is the watermark image bytes (PNG/JPEG): it
 * overrides any `image` field in the options JSON and clears the
 * default `text` (the core needs exactly one of them); pass NULL +
 * a `text` in the options for a text watermark.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `image` must be NULL
 * or point at `image_len` readable bytes. `options_json` must be
 * NULL or a NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_add_watermark(const unsigned char *pdf,
                                             uintptr_t pdf_len,
                                             const unsigned char *image,
                                             uintptr_t image_len,
                                             const char *options_json);

/**
 * Stamp page numbers over the selected pages. `options_json` is a
 * partial `PageNumberOptions` JSON object, e.g.
 * `{"format": "Page {n} of {total}", "position": "bottom_center"}`.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `options_json` must
 * be NULL or a NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_add_page_numbers(const unsigned char *pdf,
                                                uintptr_t pdf_len,
                                                const char *options_json);

/**
 * Place a handwritten signature IMAGE on one page (image placement
 * only — not a cryptographic digital signature). `image` is the
 * signature image bytes (transparent PNG recommended).
 * `options_json` is a partial `SignatureOptions` JSON object, e.g.
 * `{"page": 1, "x": 36.0, "y": 36.0, "width": 144.0}`.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `image` must point
 * at `image_len` readable bytes. `options_json` must be NULL or a
 * NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_place_signature(const unsigned char *pdf,
                                               uintptr_t pdf_len,
                                               const unsigned char *image,
                                               uintptr_t image_len,
                                               const char *options_json);

/**
 * Extract many selections from one PDF in a single call. One bad
 * selection fails only its own item, never the whole batch.
 * `selections_json` is a JSON array of page-number arrays, e.g.
 * `"[[1], [2, 3], [9]]"`. On success `data` is UTF-8 JSON: an array
 * of `{"index": n, "pdf": "<base64>"}` per success or
 * `{"index": n, "error_code": "…", "error": "…"}` per failure, in
 * input order. `options_json` is a partial `PageSetOptions` JSON object.
 *
 * # Safety
 * `pdf` must point at `pdf_len` readable bytes. `selections_json`
 * must be a NUL-terminated string. `options_json` must be NULL or a
 * NUL-terminated string.
 */
struct DusrelResult *dusrelpdf_bulk_extract(const unsigned char *pdf,
                                            uintptr_t pdf_len,
                                            const char *selections_json,
                                            const char *options_json);
