/**
 * dusrelpdf — offline PDF toolkit (TypeScript wrapper over the Rust core).
 *
 * Unlike indycheck (whose JS package is a pure-TS mirror of string
 * validators), PDF binary manipulation cannot be mirrored in pure TS,
 * so this package wraps the Rust core — through the native napi-rs
 * addon when it is built (`crates/dusrelpdf-napi`, `.node`), and
 * through WASM (`crates/dusrelpdf-wasm`, `--target nodejs`) as a
 * fallback. `backend()` reports which engine is active; setting
 * `DUSREL_PDF_FORCE_WASM=1` forces the WASM path (tests / debugging).
 * Every operation takes bytes in and returns bytes out,
 * synchronously — both backends are synchronous, so there is nothing
 * to `await` (awaiting a plain value still works, it just does nothing).
 *
 * Backend contract (see crates/dusrelpdf-wasm/src/lib.rs and
 * crates/dusrelpdf-napi/src/lib.rs):
 * - Binary results are `Uint8Array`s (splits return an Array of them).
 * - WASM failures throw a JSON string:
 *   `{"ok":false,"code":"PAGE_OUT_OF_RANGE","reason_en":"…",
 *   "reason_hi":"…"}`. Native failures throw a JS `Error` whose
 *   message is `"CODE: reason in English"`. Either way every failure
 *   surfaces here as a `DusrelError` with the same `code` /
 *   `reason_en` / `reason_hi` — never a raw TypeError, never a raw
 *   JSON string. (For native errors the Hindi reason is not carried
 *   in the message, so `reasonHi` falls back to the English reason.)
 * - Options are plain JS objects matching the Rust `*Options` structs
 *   (crates/dusrelpdf/src/options.rs). IMPORTANT: serde does NOT fill
 *   missing non-Option fields from the Rust `Default` impl — calling
 *   wasm with a partial object like `{}` fails with
 *   `INVALID_ARGUMENT: missing field 'angle'` (verified against the
 *   built wasm). So this wrapper always sends COMPLETE option objects,
 *   filling every missing field with the Rust default (see the
 *   `to*Options` translators below). Passing `undefined` / omitting a
 *   field is exactly the Rust default; passing a field overrides it.
 *   (The native binding fills missing fields from the Rust defaults
 *   itself, so it accepts partial objects too — but the wrapper sends
 *   complete ones either way, so both backends behave identically.)
 *
 * Naming: the TS API uses camelCase option keys (`fontSize`,
 * `startAt`, `pageSize`, `imageWidth`, …) and the wrapper translates
 * them to the Rust snake_case JSON keys (`font_size`, `start_at`,
 * `page_size`, `image_width`, …) before calling either backend. The
 * mapping is listed per-interface below and in the README.
 *
 * Input rule (indycheck v0.1.0 lesson, applied to bytes): functions
 * accept `Uint8Array` only. ANY non-Uint8Array input (string, number,
 * null, plain object, Buffer-free pretenders) throws a `DusrelError`
 * with code `INVALID_TYPE` — never a raw TypeError. (A Node `Buffer`
 * IS a `Uint8Array` subclass, so Buffers are accepted as-is.)
 *
 * Honest limits (same as the Rust core, v0.1.0): no PDF rendering
 * (PDF → image), no tolerant repair of malformed PDFs, merge drops
 * the original outline trees (rebuilt as flat first-page bookmarks)
 * and does not carry AcroForm / named destinations / embedded files,
 * watermark / page-number text is ASCII only (base-14 fonts), and
 * signature placement is a raster IMAGE on the page — not a
 * cryptographic digital signature.
 */

import * as path from "node:path";

// The wasm-pack `--target nodejs` output. Built by `npm run build`
// (wasm-pack first, then tsc); gitignored — see packages/js/.gitignore.
// Loaded lazily (see `loadBackend` below) so the package also works
// when only the native `.node` addon is built.
const WASM_ENTRY = "../wasm-pkg/dusrelpdf_wasm";

// ---------------------------------------------------------------------------
// Backend selection — native napi-rs addon when built, WASM fallback.
// ---------------------------------------------------------------------------

/** Which engine is actually running the Rust core. */
export type DusrelBackend = "native" | "wasm";

/**
 * The low-level surface both backends expose. The WASM module uses
 * snake_case names, the native addon camelCase (`js_name`s); the
 * adapters below normalise that. Binary arguments are `Uint8Array`s
 * on this side; the native adapter converts them to Node `Buffer`s
 * (napi `Buffer` requires a real Buffer, not a plain Uint8Array).
 */
interface BackendImpl {
  version(): string;
  make_test_pdf(pages: number): Uint8Array;
  merge_pdfs(inputs: Uint8Array[], options: Record<string, unknown>): Uint8Array;
  split_pdf(
    pdf: Uint8Array,
    ranges: number[][],
    options: Record<string, unknown>,
  ): Uint8Array[];
  split_pdf_ranges(
    pdf: Uint8Array,
    ranges: string,
    options: Record<string, unknown>,
  ): Uint8Array[];
  split_each_page(
    pdf: Uint8Array,
    options: Record<string, unknown>,
  ): Uint8Array[];
  extract_pages(
    pdf: Uint8Array,
    pages: number[],
    options: Record<string, unknown>,
  ): Uint8Array;
  extract_pages_range(
    pdf: Uint8Array,
    range: string,
    options: Record<string, unknown>,
  ): Uint8Array;
  remove_pages(
    pdf: Uint8Array,
    pages: number[],
    options: Record<string, unknown>,
  ): Uint8Array;
  reorder_pages(
    pdf: Uint8Array,
    order: number[],
    options: Record<string, unknown>,
  ): Uint8Array;
  rotate_pages(pdf: Uint8Array, options: Record<string, unknown>): Uint8Array;
  images_to_pdf(
    images: Uint8Array[],
    options: Record<string, unknown>,
  ): Uint8Array;
  add_watermark(pdf: Uint8Array, options: Record<string, unknown>): Uint8Array;
  add_page_numbers(
    pdf: Uint8Array,
    options: Record<string, unknown>,
  ): Uint8Array;
  place_signature(
    pdf: Uint8Array,
    signature: Uint8Array,
    options: Record<string, unknown>,
  ): Uint8Array;
  get_pdf_info(pdf: Uint8Array, options: Record<string, unknown>): string;
  bulk_extract(
    pdf: Uint8Array,
    selections: number[][],
    options: Record<string, unknown>,
  ): string;
}

type AnyFn = (...args: any[]) => any;
type AnyRecord = Record<string, AnyFn>;

function makeWasmBackend(mod: AnyRecord): BackendImpl {
  return {
    version: () => mod["version"]() as string,
    make_test_pdf: (pages) => mod["make_test_pdf"](pages) as Uint8Array,
    merge_pdfs: (inputs, options) =>
      mod["merge_pdfs"](inputs, options) as Uint8Array,
    split_pdf: (pdf, ranges, options) =>
      mod["split_pdf"](pdf, ranges, options) as Uint8Array[],
    split_pdf_ranges: (pdf, ranges, options) =>
      mod["split_pdf_ranges"](pdf, ranges, options) as Uint8Array[],
    split_each_page: (pdf, options) =>
      mod["split_each_page"](pdf, options) as Uint8Array[],
    extract_pages: (pdf, pages, options) =>
      mod["extract_pages"](pdf, pages, options) as Uint8Array,
    extract_pages_range: (pdf, range, options) =>
      mod["extract_pages_range"](pdf, range, options) as Uint8Array,
    remove_pages: (pdf, pages, options) =>
      mod["remove_pages"](pdf, pages, options) as Uint8Array,
    reorder_pages: (pdf, order, options) =>
      mod["reorder_pages"](pdf, order, options) as Uint8Array,
    rotate_pages: (pdf, options) =>
      mod["rotate_pages"](pdf, options) as Uint8Array,
    images_to_pdf: (images, options) =>
      mod["images_to_pdf"](images, options) as Uint8Array,
    add_watermark: (pdf, options) =>
      mod["add_watermark"](pdf, options) as Uint8Array,
    add_page_numbers: (pdf, options) =>
      mod["add_page_numbers"](pdf, options) as Uint8Array,
    place_signature: (pdf, signature, options) =>
      mod["place_signature"](pdf, signature, options) as Uint8Array,
    get_pdf_info: (pdf, options) =>
      mod["get_pdf_info"](pdf, options) as string,
    bulk_extract: (pdf, selections, options) =>
      mod["bulk_extract"](pdf, selections, options) as string,
  };
}

/** A Node `Buffer` for napi, unchanged when it already is one. */
function toBuffer(value: Uint8Array): Buffer {
  return Buffer.isBuffer(value) ? value : Buffer.from(value);
}

function makeNativeBackend(mod: AnyRecord): BackendImpl {
  const bufs = (inputs: Uint8Array[]): Buffer[] => inputs.map(toBuffer);
  return {
    version: () => mod["version"]() as string,
    make_test_pdf: (pages) => mod["makeTestPdf"](pages) as Uint8Array,
    merge_pdfs: (inputs, options) =>
      mod["mergePdfs"](bufs(inputs), options) as Uint8Array,
    split_pdf: (pdf, ranges, options) =>
      mod["splitPdf"](toBuffer(pdf), ranges, options) as Uint8Array[],
    split_pdf_ranges: (pdf, ranges, options) =>
      mod["splitPdfRanges"](toBuffer(pdf), ranges, options) as Uint8Array[],
    split_each_page: (pdf, options) =>
      mod["splitEachPage"](toBuffer(pdf), options) as Uint8Array[],
    extract_pages: (pdf, pages, options) =>
      mod["extractPages"](toBuffer(pdf), pages, options) as Uint8Array,
    extract_pages_range: (pdf, range, options) =>
      mod["extractPagesRange"](toBuffer(pdf), range, options) as Uint8Array,
    remove_pages: (pdf, pages, options) =>
      mod["removePages"](toBuffer(pdf), pages, options) as Uint8Array,
    reorder_pages: (pdf, order, options) =>
      mod["reorderPages"](toBuffer(pdf), order, options) as Uint8Array,
    rotate_pages: (pdf, options) =>
      mod["rotatePages"](toBuffer(pdf), options) as Uint8Array,
    images_to_pdf: (images, options) =>
      mod["imagesToPdf"](bufs(images), options) as Uint8Array,
    add_watermark: (pdf, options) =>
      mod["addWatermark"](toBuffer(pdf), options) as Uint8Array,
    add_page_numbers: (pdf, options) =>
      mod["addPageNumbers"](toBuffer(pdf), options) as Uint8Array,
    place_signature: (pdf, signature, options) =>
      mod["placeSignature"](toBuffer(pdf), toBuffer(signature), options) as Uint8Array,
    get_pdf_info: (pdf, options) =>
      mod["getPdfInfo"](toBuffer(pdf), options) as string,
    bulk_extract: (pdf, selections, options) =>
      mod["bulkExtract"](toBuffer(pdf), selections, options) as string,
  };
}

/** `DUSREL_PDF_FORCE_WASM=1` forces the WASM path (tests / debugging). */
function forceWasm(): boolean {
  return process.env.DUSREL_PDF_FORCE_WASM === "1";
}

/** Platform triple used in napi `.node` filenames, e.g.
 *  `linux-x64-gnu` (napi CLI output: `dusrelpdf-napi.<triple>.node`). */
function nativeTriple(): string {
  const platform = process.platform;
  const arch = process.arch;
  if (platform === "linux") return `linux-${arch}-gnu`;
  if (platform === "win32") return `win32-${arch}-msvc`;
  return `${platform}-${arch}`;
}

/** Candidate `.node` paths, in priority order. `DUSREL_PDF_NATIVE_PATH`
 *  points straight at one file (tests / custom builds). The rest
 *  search `crates/dusrelpdf-napi/` relative to this file (dist build
 *  and vitest both sit two levels below the repo root) and relative
 *  to the working directory (repo root, or `packages/js`). */
function nativeCandidates(): string[] {
  const triple = nativeTriple();
  const names = [
    `dusrelpdf-napi.${triple}.node`,
    `index.${triple}.node`,
    "dusrelpdf-napi.node",
    "index.node",
  ];
  const dirs: string[] = [];
  if (typeof __dirname === "string") {
    dirs.push(path.join(__dirname, "..", "..", "crates", "dusrelpdf-napi"));
  }
  dirs.push(path.join(process.cwd(), "crates", "dusrelpdf-napi"));
  dirs.push(path.join(process.cwd(), "..", "..", "crates", "dusrelpdf-napi"));
  const out: string[] = [];
  const override = process.env.DUSREL_PDF_NATIVE_PATH;
  if (override) out.push(override);
  for (const dir of dirs) {
    for (const name of names) out.push(path.join(dir, name));
  }
  return out;
}

/** Try to load the native addon; `null` when it is not built (or the
 *  platform/arch does not match — a wrong `.node` throws on require
 *  and is skipped, so a stale binary never breaks the WASM fallback). */
function loadNative(): BackendImpl | null {
  if (forceWasm()) return null;
  if (typeof require === "undefined") return null;
  for (const candidate of nativeCandidates()) {
    try {
      const mod = require(candidate) as AnyRecord;
      if (mod && typeof mod["mergePdfs"] === "function") {
        return makeNativeBackend(mod);
      }
    } catch {
      // Missing file, or a `.node` for another platform — try next.
    }
  }
  return null;
}

/** Load the WASM build (throws if `wasm-pkg` was never built — run
 *  `npm run build` in `packages/js`). */
function loadWasm(): BackendImpl {
  // require, not a static import: the file may not exist when only
  // the native addon is built, and a static import would fail at
  // load time instead of falling through to the error here.
  const wasm = require(WASM_ENTRY) as AnyRecord;
  return makeWasmBackend(wasm);
}

interface BackendCache {
  name: DusrelBackend;
  impl: BackendImpl;
}

let backendCache: BackendCache | null = null;

function loadBackend(): BackendCache {
  const native = loadNative();
  if (native) return { name: "native", impl: native };
  return { name: "wasm", impl: loadWasm() };
}

function backendImpl(): BackendImpl {
  if (!backendCache) backendCache = loadBackend();
  return backendCache.impl;
}

/**
 * Which engine is actually running the Rust core: `"native"` (the
 * napi-rs `.node` addon, when built in `crates/dusrelpdf-napi`) or
 * `"wasm"` (the wasm-pack fallback, or when `DUSREL_PDF_FORCE_WASM=1`
 * is set). Loads the backend on first call.
 */
export function backend(): DusrelBackend {
  if (!backendCache) backendCache = loadBackend();
  return backendCache.name;
}


// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/** Stable error codes — verbatim from crates/dusrelpdf/src/error.rs. */
export type DusrelErrorCode =
  | "INVALID_PDF"
  | "ENCRYPTED_PDF"
  | "PAGE_OUT_OF_RANGE"
  | "EMPTY_INPUT"
  | "INVALID_RANGE"
  | "INVALID_ANGLE"
  | "INVALID_IMAGE"
  | "INVALID_ARGUMENT"
  | "INVALID_TYPE"
  | "OPERATION_FAILED";

/**
 * Every failure from this package. `code` is stable and
 * machine-readable; `reasonEn` / `reasonHi` are the human-readable
 * reasons from the Rust core (Roman-script Hinglish, like indycheck).
 */
export class DusrelError extends Error {
  readonly code: DusrelErrorCode;
  readonly reasonEn: string;
  readonly reasonHi: string;

  constructor(code: DusrelErrorCode, reasonEn: string, reasonHi: string) {
    super(`${code}: ${reasonEn}`);
    this.name = "DusrelError";
    this.code = code;
    this.reasonEn = reasonEn;
    this.reasonHi = reasonHi;
  }
}

function invalidType(what: string): DusrelError {
  return new DusrelError(
    "INVALID_TYPE",
    `${what} must be a Uint8Array.`,
    `${what} Uint8Array hona chahiye.`,
  );
}

function invalidArgument(reasonEn: string, reasonHi: string): DusrelError {
  return new DusrelError("INVALID_ARGUMENT", reasonEn, reasonHi);
}

/** Turn whatever a backend threw into a DusrelError (pass-through if it
 *  already is one, e.g. from the client-side checks below). */
function toDusrelError(e: unknown): DusrelError {
  if (e instanceof DusrelError) return e;
  if (typeof e === "string") {
    try {
      const parsed = JSON.parse(e) as {
        code?: unknown;
        reason_en?: unknown;
        reason_hi?: unknown;
      };
      if (parsed && typeof parsed.code === "string") {
        return new DusrelError(
          parsed.code as DusrelErrorCode,
          typeof parsed.reason_en === "string"
            ? parsed.reason_en
            : "Operation failed.",
          typeof parsed.reason_hi === "string"
            ? parsed.reason_hi
            : "Operation fail ho gaya.",
        );
      }
    } catch {
      // Not JSON — fall through to the plain-string case below.
    }
    return new DusrelError("OPERATION_FAILED", e, e);
  }
  if (e instanceof Error) {
    // Native (.node) errors carry the code in the message:
    // "INVALID_RANGE: …". The Hindi reason is not carried in that
    // message, so reasonHi falls back to the English reason.
    const m = /^([A-Z][A-Z0-9_]*):\s([\s\S]*)$/.exec(e.message);
    if (m && ERROR_CODES.has(m[1])) {
      return new DusrelError(m[1] as DusrelErrorCode, m[2], m[2]);
    }
    return new DusrelError("OPERATION_FAILED", e.message, e.message);
  }
  return new DusrelError(
    "OPERATION_FAILED",
    "Unknown error.",
    "Unknown error hai.",
  );
}

/** All known error codes (crates/dusrelpdf/src/error.rs), for the
 *  native-error parse above. */
const ERROR_CODES: ReadonlySet<string> = new Set([
  "INVALID_PDF",
  "ENCRYPTED_PDF",
  "PAGE_OUT_OF_RANGE",
  "EMPTY_INPUT",
  "INVALID_RANGE",
  "INVALID_ANGLE",
  "INVALID_IMAGE",
  "INVALID_ARGUMENT",
  "INVALID_TYPE",
  "OPERATION_FAILED",
]);

/** Run a wasm call, converting any throw into a DusrelError. */
function call<T>(fn: () => T): T {
  try {
    return fn();
  } catch (e) {
    throw toDusrelError(e);
  }
}

// ---------------------------------------------------------------------------
// Options — camelCase TS keys, translated to Rust snake_case (see README
// for the full mapping table). Every field is optional: omitted fields
// use the Rust defaults listed in the doc comments below.
// ---------------------------------------------------------------------------

/** Options shared by every operation that reads a PDF. */
export interface CommonOptions {
  /** Password for an encrypted PDF. Empty-password PDFs open without it. */
  password?: string;
  /** Compress output streams (default true). Set false for faster saves. */
  compress?: boolean;
}

/** Options for {@link mergePdfs}. */
export interface MergeOptions extends CommonOptions {
  /** Copy the first input's document info (title/author) into the
   *  output (default true). Outlines are always rebuilt as flat
   *  first-page bookmarks. */
  preserveMetadata?: boolean;
  /** Add a flat outline bookmark for the first page of each input
   *  (default true). */
  addBookmarks?: boolean;
}

/** Options for split / extract / remove / reorder — just the shared
 *  options (password / compress). */
export type PageSetOptions = CommonOptions;
export type SplitOptions = CommonOptions;
export type ExtractOptions = CommonOptions;
export type RemoveOptions = CommonOptions;
export type ReorderOptions = CommonOptions;

/** Where on the page a stamp (watermark / page number) goes. */
export type Position =
  | "top_left"
  | "top_center"
  | "top_right"
  | "bottom_left"
  | "bottom_center"
  | "bottom_right"
  | "center";

/** A base-14 (built-in) PDF font. No embedding needed, so these always
 *  work; only WinAnsi (Latin) text is supported. */
export type BaseFont =
  | "helvetica"
  | "helvetica_bold"
  | "times_roman"
  | "courier"
  | "courier_bold";

/** Options for {@link rotatePages}. */
export interface RotateOptions extends CommonOptions {
  /**
   * Rotation in degrees; must be a multiple of 90 (default 90).
   * Negative angles are normalised (e.g. -90 becomes 270).
   */
  angle?: number;
  /** Pages to rotate, 1-based. Omitted = every page. */
  pages?: number[];
}

/** Page size for {@link imagesToPdf}. A plain string picks a named
 *  size; `{ width, height }` is a custom size in points. */
export type PageSize =
  | "auto"
  | "a4"
  | "letter"
  | { width: number; height: number };

/** How an image is fitted into its page. */
export type ImageFit = "contain" | "fill";

/** Options for {@link imagesToPdf}. */
export interface ImagesToPdfOptions {
  /** Page size (default "a4" with "contain" fit). "auto" sizes each
   *  page to its image (1 pixel = 1 point). */
  pageSize?: PageSize;
  /** Fit mode (default "contain": scale to fit, keeping aspect ratio;
   *  "fill" stretches to the whole page). */
  fit?: ImageFit;
  /** Background colour as [r, g, b] 0.0–1.0 for letterboxed areas
   *  (default white [1, 1, 1]). */
  background?: [number, number, number];
}

/** Options for {@link addWatermark}. */
export interface WatermarkOptions extends CommonOptions {
  /**
   * Watermark text. Mutually exclusive with `image` — set one.
   * Default "DRAFT" when neither `text` nor `image` is given.
   */
  text?: string;
  /**
   * Watermark image bytes (PNG/JPEG). Mutually exclusive with `text`.
   * Sent to wasm as a plain JS number array (serde-wasm-bindgen
   * cannot decode a Uint8Array into Vec<u8>).
   */
  image?: Uint8Array;
  /** Opacity 0.0–1.0 (default 0.3). */
  opacity?: number;
  /** Rotation in degrees, clockwise (default 45). Pass 0 for an
   *  unrotated image watermark. */
  rotation?: number;
  /** Font size in points (text only, default 48). */
  fontSize?: number;
  /** Font (text only, default "helvetica"). */
  font?: BaseFont;
  /** Where to place it (default "center"). */
  position?: Position;
  /** Custom position in points from the bottom-left corner. Used only
   *  when both `x` and `y` are set. */
  x?: number;
  y?: number;
  /** Pages to watermark, 1-based. Omitted = every page. */
  pages?: number[];
  /** Image width/height in points (image watermarks only).
   *  Omitted = natural size at 96 dpi. */
  imageWidth?: number;
  imageHeight?: number;
}

/** Options for {@link addPageNumbers}. */
export interface PageNumberOptions extends CommonOptions {
  /** Template with `{n}` for the page number and `{total}` for the
   *  total, e.g. `"{n}"`, `"Page {n} of {total}"` (default `"{n}"`).
   *  Must contain `{n}`. */
  format?: string;
  /** Where to place the number (default "bottom_center"). */
  position?: Position;
  /** First printed number (default 1). Use 0 to start from 0. */
  startAt?: number;
  /** Font size in points (default 10). */
  fontSize?: number;
  /** Font (default "helvetica"). */
  font?: BaseFont;
  /** Margin from the page edge in points (default 36). */
  margin?: number;
  /** Pages to number, 1-based. Omitted = every page. */
  pages?: number[];
}

/** Options for {@link placeSignature}. */
export interface SignatureOptions extends CommonOptions {
  /** Target page, 1-based (default 1). */
  page?: number;
  /** Position in points from the bottom-left corner (default 36, 36). */
  x?: number;
  y?: number;
  /** Display size in points. Omitted width = 144 pt (2 inches) with
   *  height from the image aspect ratio. */
  width?: number;
  height?: number;
}

/** Summary of a PDF document, from {@link getPdfInfo}. */
export interface PdfInfo {
  /** Number of pages. */
  pageCount: number;
  /** PDF version string, e.g. `"1.7"`. */
  version: string;
  /** True when the trailer has an `/Encrypt` entry. */
  encrypted: boolean;
  /** Document title from the Info dict, if present. */
  title?: string;
  /** Page sizes in points (width, height), in page order. */
  pageSizes: Array<[number, number]>;
}

/** One item of a {@link bulkExtract} result, in input order. */
export interface BulkExtractItem {
  /** Input index (0-based). */
  index: number;
  /** Output bytes on success; absent on failure. */
  pdf?: Uint8Array;
  /** Error code on failure (e.g. "PAGE_OUT_OF_RANGE"); absent on success. */
  errorCode?: string;
  /** Human-readable error (English) on failure; absent on success. */
  error?: string;
}

// ---------------------------------------------------------------------------
// Client-side validation (cheap checks; the core does the real ones)
// ---------------------------------------------------------------------------

/** Assert a value is a Uint8Array (Buffers pass: Buffer extends
 *  Uint8Array). Empty bytes are EMPTY_INPUT, like the Rust core. */
function assertPdfBytes(value: unknown, what = "PDF bytes"): Uint8Array {
  if (!(value instanceof Uint8Array)) throw invalidType(what);
  if (value.byteLength === 0) {
    throw new DusrelError(
      "EMPTY_INPUT",
      `${what} is empty.`,
      `${what} khaali hai.`,
    );
  }
  return value;
}

/** Assert an array of Uint8Arrays (merge / images-to-PDF inputs). */
function assertPdfArray(value: unknown, what: string, emptyWhat: string): Uint8Array[] {
  if (!Array.isArray(value)) throw invalidType(what);
  if (value.length === 0) {
    throw new DusrelError(
      "EMPTY_INPUT",
      `${emptyWhat} is empty.`,
      `${emptyWhat} khaali hai.`,
    );
  }
  for (const item of value) {
    if (!(item instanceof Uint8Array)) throw invalidType(what);
  }
  return value as Uint8Array[];
}

/** Assert a page-number list: an array of integers. Values 0 / negative
 *  / above the page count are NOT rejected here — the core reports
 *  them as PAGE_OUT_OF_RANGE. Non-integers are a caller error. */
function assertPageList(value: unknown, what = "Page list"): number[] {
  if (!Array.isArray(value)) {
    throw new DusrelError(
      "INVALID_TYPE",
      `${what} must be an array of page numbers.`,
      `${what} page numbers ki array honi chahiye.`,
    );
  }
  for (const page of value) {
    if (typeof page !== "number" || !Number.isInteger(page)) {
      throw invalidArgument(
        "Page numbers must be integers.",
        "Page numbers integer hone chahiye.",
      );
    }
  }
  return value as number[];
}

/** Assert a range string ("1-3,5"). Empty / malformed strings are
 *  INVALID_RANGE, exactly like the Rust core; non-strings are
 *  INVALID_TYPE. */
function assertRangeString(value: unknown): string {
  if (typeof value !== "string") {
    throw new DusrelError(
      "INVALID_TYPE",
      "Page range must be a string, e.g. \"1-3,5\".",
      "Page range string hona chahiye, jaise \"1-3,5\".",
    );
  }
  if (value.trim() === "") {
    throw new DusrelError(
      "INVALID_RANGE",
      `Invalid page range "${value}". Use forms like "1-3,5" with pages numbered from 1.`,
      `Galat page range "${value}" hai. "1-3,5" jaise form use karein, pages 1 se number hote hain.`,
    );
  }
  return value;
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

// ---------------------------------------------------------------------------
// Options translation — camelCase TS -> complete snake_case Rust objects.
// Every translator fills ALL Rust-required fields with the Rust
// defaults, because serde rejects partial objects (see header).
// Undefined optional fields are omitted from the object sent to the backend.
// ---------------------------------------------------------------------------

function toCommon(options?: CommonOptions): Record<string, unknown> {
  const out: Record<string, unknown> = {
    compress: options?.compress ?? true,
  };
  if (options?.password !== undefined) out["password"] = options.password;
  return out;
}

function toMergeOptions(options?: MergeOptions): Record<string, unknown> {
  return {
    ...toCommon(options),
    preserve_metadata: options?.preserveMetadata ?? true,
    add_bookmarks: options?.addBookmarks ?? true,
  };
}

function toRotateOptions(options?: RotateOptions): Record<string, unknown> {
  const angle = options?.angle ?? 90;
  if (!Number.isInteger(angle) || angle % 90 !== 0) {
    // Same code + messages as Rust DusrelError::invalid_angle.
    throw new DusrelError(
      "INVALID_ANGLE",
      `Rotation angle ${angle} is not a multiple of 90 degrees. Use 0, 90, 180 or 270.`,
      `Rotation angle ${angle} 90 ka multiple nahi hai. 0, 90, 180 ya 270 use karein.`,
    );
  }
  const out: Record<string, unknown> = { ...toCommon(options), angle };
  if (options?.pages !== undefined) out["pages"] = assertPageList(options.pages);
  return out;
}

function toWatermarkOptions(options?: WatermarkOptions): Record<string, unknown> {
  const hasImage = options?.image !== undefined;
  const hasText =
    options?.text !== undefined && options.text !== "";
  if (hasImage && hasText) {
    throw invalidArgument(
      "Watermark needs exactly one of `text` or `image`.",
      "Watermark ke liye `text` ya `image` me se exactly ek chahiye.",
    );
  }
  if (hasImage && !(options?.image instanceof Uint8Array)) {
    throw invalidType("Watermark image");
  }
  const opacity = options?.opacity ?? 0.3;
  if (!isFiniteNumber(opacity) || opacity < 0 || opacity > 1) {
    throw invalidArgument(
      "Opacity must be between 0.0 and 1.0.",
      "Opacity 0.0 se 1.0 ke beech honi chahiye.",
    );
  }
  const out: Record<string, unknown> = {
    ...toCommon(options),
    opacity,
    rotation: options?.rotation ?? 45,
    font_size: options?.fontSize ?? 48,
    font: options?.font ?? "helvetica",
    position: options?.position ?? "center",
  };
  if (hasImage) {
    // Plain JS number array, NOT a Uint8Array: serde-wasm-bindgen
    // cannot decode a Uint8Array into Vec<u8> (wasm lib.rs also has
    // a manual fallback for this field — the array form is canonical).
    out["image"] = Array.from(options!.image as Uint8Array);
  } else {
    out["text"] = options?.text ?? "DRAFT";
  }
  if (options?.x !== undefined) out["x"] = options.x;
  if (options?.y !== undefined) out["y"] = options.y;
  if (options?.pages !== undefined) out["pages"] = assertPageList(options.pages);
  if (options?.imageWidth !== undefined) out["image_width"] = options.imageWidth;
  if (options?.imageHeight !== undefined) out["image_height"] = options.imageHeight;
  return out;
}

function toPageNumberOptions(options?: PageNumberOptions): Record<string, unknown> {
  const format = options?.format ?? "{n}";
  if (!format.includes("{n}")) {
    throw invalidArgument(
      'Page-number format must contain {n}, e.g. "{n}" or "Page {n} of {total}".',
      'Page-number format me {n} hona chahiye, jaise "{n}" ya "Page {n} of {total}".',
    );
  }
  const out: Record<string, unknown> = {
    ...toCommon(options),
    format,
    position: options?.position ?? "bottom_center",
    start_at: options?.startAt ?? 1,
    font_size: options?.fontSize ?? 10,
    font: options?.font ?? "helvetica",
    margin: options?.margin ?? 36,
  };
  if (options?.pages !== undefined) out["pages"] = assertPageList(options.pages);
  return out;
}

function toPageSize(pageSize: PageSize): unknown {
  if (typeof pageSize === "string") return pageSize;
  if (
    typeof pageSize === "object" &&
    pageSize !== null &&
    isFiniteNumber(pageSize.width) &&
    isFiniteNumber(pageSize.height)
  ) {
    // Serde struct-variant form: {"custom": {"width": …, "height": …}}.
    return { custom: { width: pageSize.width, height: pageSize.height } };
  }
  throw invalidArgument(
    'Page size must be "auto", "a4", "letter", or { width, height }.',
    'Page size "auto", "a4", "letter", ya { width, height } hona chahiye.',
  );
}

function toImagesOptions(options?: ImagesToPdfOptions): Record<string, unknown> {
  const background = options?.background ?? [1, 1, 1];
  if (
    !Array.isArray(background) ||
    background.length !== 3 ||
    !background.every(isFiniteNumber)
  ) {
    throw invalidArgument(
      "Background must be [r, g, b] with numbers 0.0–1.0.",
      "Background [r, g, b] hona chahiye, numbers 0.0–1.0 ke beech.",
    );
  }
  return {
    page_size: toPageSize(options?.pageSize ?? "a4"),
    fit: options?.fit ?? "contain",
    background: [...background] as [number, number, number],
  };
}

function toSignatureOptions(options?: SignatureOptions): Record<string, unknown> {
  const page = options?.page ?? 1;
  if (!Number.isInteger(page)) {
    throw invalidArgument(
      "Signature page must be an integer.",
      "Signature page integer hona chahiye.",
    );
  }
  const out: Record<string, unknown> = {
    ...toCommon(options),
    page,
    x: options?.x ?? 36,
    y: options?.y ?? 36,
  };
  if (options?.width !== undefined) out["width"] = options.width;
  if (options?.height !== undefined) out["height"] = options.height;
  return out;
}

// ---------------------------------------------------------------------------
// Wasm result helpers
// ---------------------------------------------------------------------------

function asBytes(value: unknown): Uint8Array {
  if (value instanceof Uint8Array) return value;
  // wasm-bindgen returns Uint8Array for Vec<u8>; anything else is a bug.
  throw new DusrelError(
    "OPERATION_FAILED",
    "Backend returned a non-bytes result.",
    "WASM ne bytes result nahi diya.",
  );
}

function asBytesArray(value: unknown): Uint8Array[] {
  if (!Array.isArray(value)) {
    throw new DusrelError(
      "OPERATION_FAILED",
      "Backend returned a non-array result.",
      "WASM ne array result nahi diya.",
    );
  }
  return value.map((item) => asBytes(item));
}

function base64ToBytes(b64: string): Uint8Array {
  // Node-only package (wasm --target nodejs); Buffer is available.
  return new Uint8Array(Buffer.from(b64, "base64"));
}

// ---------------------------------------------------------------------------
// Public API — all synchronous, all throw DusrelError on failure.
// ---------------------------------------------------------------------------

/** Crate version, e.g. `"0.1.0"` (lockstep with npm / PyPI / crates.io). */
export function version(): string {
  return call(() => backendImpl().version());
}

/**
 * Build a mock multi-page PDF (test data only). Every page says
 * `Page <n>` in Courier on A4. `pages` below 1 is clamped to 1 by
 * the Rust core, like `make_test_pdf(0)` returning a 1-page PDF.
 */
export function makeTestPdf(pages: number): Uint8Array {
  if (typeof pages !== "number" || !Number.isInteger(pages) || pages < 0) {
    throw invalidArgument(
      "Test PDF page count must be a non-negative integer.",
      "Test PDF page count non-negative integer hona chahiye.",
    );
  }
  return call(() => asBytes(backendImpl().make_test_pdf(pages)));
}

/** Merge multiple PDFs into one, in input order. */
export function mergePdfs(inputs: unknown, options?: MergeOptions): Uint8Array {
  const bufs = assertPdfArray(inputs, "Merge inputs", "merge inputs");
  return call(() => asBytes(backendImpl().merge_pdfs(bufs, toMergeOptions(options))));
}

/** Split one PDF into one output per range (arrays of 1-based page
 *  numbers, e.g. `[[1, 2], [3]]`). */
export function splitPdf(
  pdf: unknown,
  ranges: unknown,
  options?: SplitOptions,
): Uint8Array[] {
  const bytes = assertPdfBytes(pdf);
  if (!Array.isArray(ranges)) {
    throw new DusrelError(
      "INVALID_TYPE",
      "Split ranges must be an array of page-number arrays.",
      "Split ranges page-number arrays ki list honi chahiye.",
    );
  }
  if (ranges.length === 0) {
    throw new DusrelError(
      "EMPTY_INPUT",
      "split ranges is empty.",
      "split ranges khaali hai.",
    );
  }
  const parsed = ranges.map((r) => assertPageList(r, "Split range"));
  return call(() =>
    asBytesArray(backendImpl().split_pdf(bytes, parsed, toCommon(options))),
  );
}

/** Split from a range string such as `"1-3,4-5"` — one output per
 *  comma-separated chunk; a bare number is a one-page chunk. */
export function splitPdfRanges(
  pdf: unknown,
  ranges: unknown,
  options?: SplitOptions,
): Uint8Array[] {
  const bytes = assertPdfBytes(pdf);
  const spec = assertRangeString(ranges);
  return call(() =>
    asBytesArray(backendImpl().split_pdf_ranges(bytes, spec, toCommon(options))),
  );
}

/** Split into one PDF per page, in page order. */
export function splitEachPage(pdf: unknown, options?: SplitOptions): Uint8Array[] {
  const bytes = assertPdfBytes(pdf);
  return call(() =>
    asBytesArray(backendImpl().split_each_page(bytes, toCommon(options))),
  );
}

/** Extract selected pages into a new PDF, in the order given
 *  (duplicates repeat the page). */
export function extractPages(
  pdf: unknown,
  pages: unknown,
  options?: ExtractOptions,
): Uint8Array {
  const bytes = assertPdfBytes(pdf);
  const list = assertPageList(pages);
  if (list.length === 0) {
    throw new DusrelError(
      "EMPTY_INPUT",
      "page selection is empty.",
      "page selection khaali hai.",
    );
  }
  return call(() =>
    asBytes(backendImpl().extract_pages(bytes, list, toCommon(options))),
  );
}

/** Extract from a range string such as `"1-3,5"`. */
export function extractPagesRange(
  pdf: unknown,
  range: unknown,
  options?: ExtractOptions,
): Uint8Array {
  const bytes = assertPdfBytes(pdf);
  const spec = assertRangeString(range);
  return call(() =>
    asBytes(backendImpl().extract_pages_range(bytes, spec, toCommon(options))),
  );
}

/** Delete pages. An empty list is a no-op (returns the input bytes
 *  unchanged), matching the Rust core. */
export function removePages(
  pdf: unknown,
  pages: unknown,
  options?: RemoveOptions,
): Uint8Array {
  const bytes = assertPdfBytes(pdf);
  const list = assertPageList(pages);
  return call(() =>
    asBytes(backendImpl().remove_pages(bytes, list, toCommon(options))),
  );
}

/** Change the page order. `order` must list every page exactly once
 *  (a full permutation, 1-based) — otherwise INVALID_ARGUMENT. */
export function reorderPages(
  pdf: unknown,
  order: unknown,
  options?: ReorderOptions,
): Uint8Array {
  const bytes = assertPdfBytes(pdf);
  const list = assertPageList(order, "Reorder list");
  return call(() =>
    asBytes(backendImpl().reorder_pages(bytes, list, toCommon(options))),
  );
}

/** Rotate pages by a multiple of 90 degrees. `options.pages` omitted
 *  = every page; `angle` defaults to 90. */
export function rotatePages(pdf: unknown, options?: RotateOptions): Uint8Array {
  const bytes = assertPdfBytes(pdf);
  return call(() =>
    asBytes(backendImpl().rotate_pages(bytes, toRotateOptions(options))),
  );
}

/**
 * Make a PDF from images (PNG/JPEG), one page per image, in input
 * order. Transparent PNGs keep their alpha.
 */
export function imagesToPdf(
  images: unknown,
  options?: ImagesToPdfOptions,
): Uint8Array {
  const bufs = assertPdfArray(images, "Images", "images");
  return call(() => asBytes(backendImpl().images_to_pdf(bufs, toImagesOptions(options))));
}

/**
 * Stamp a text or image watermark over the selected pages. Default is
 * the text `"DRAFT"` at 45° in the centre. Pass `{ image }` for an
 * image watermark (and `rotation: 0` unless you want it rotated).
 */
export function addWatermark(pdf: unknown, options?: WatermarkOptions): Uint8Array {
  const bytes = assertPdfBytes(pdf);
  return call(() =>
    asBytes(backendImpl().add_watermark(bytes, toWatermarkOptions(options))),
  );
}

/**
 * Stamp page numbers. `{n}` is the printed number and `{total}` the
 * page count, e.g. `"Page {n} of {total}"`. Numbers are baked into the
 * content stream — re-running duplicates them.
 */
export function addPageNumbers(
  pdf: unknown,
  options?: PageNumberOptions,
): Uint8Array {
  const bytes = assertPdfBytes(pdf);
  return call(() =>
    asBytes(backendImpl().add_page_numbers(bytes, toPageNumberOptions(options))),
  );
}

/**
 * Place a handwritten signature IMAGE on one page (default page 1 at
 * 36, 36 pt, 144 pt wide). This is image placement only — NOT a
 * cryptographic digital signature. Transparent PNGs keep their
 * transparency via an `/SMask` soft-mask.
 */
export function placeSignature(
  pdf: unknown,
  signature: unknown,
  options?: SignatureOptions,
): Uint8Array {
  const bytes = assertPdfBytes(pdf);
  if (!(signature instanceof Uint8Array)) {
    throw invalidType("Signature image");
  }
  return call(() =>
    asBytes(
      backendImpl().place_signature(bytes, signature, toSignatureOptions(options)),
    ),
  );
}

/** Read a PDF's summary without modifying it. */
export function getPdfInfo(pdf: unknown, options?: CommonOptions): PdfInfo {
  const bytes = assertPdfBytes(pdf);
  const json = call(() => backendImpl().get_pdf_info(bytes, toCommon(options)));
  const parsed = JSON.parse(json) as {
    page_count: number;
    version: string;
    encrypted: boolean;
    title?: string;
    page_sizes: Array<[number, number]>;
  };
  const info: PdfInfo = {
    pageCount: parsed.page_count,
    version: parsed.version,
    encrypted: parsed.encrypted,
    pageSizes: parsed.page_sizes,
  };
  if (parsed.title !== undefined) info.title = parsed.title;
  return info;
}

/**
 * Extract many selections from one PDF in a single call. One bad
 * selection fails only its own item (with `errorCode` / `error`),
 * never the whole batch.
 */
export function bulkExtract(
  pdf: unknown,
  selections: unknown,
  options?: ExtractOptions,
): BulkExtractItem[] {
  const bytes = assertPdfBytes(pdf);
  if (!Array.isArray(selections)) {
    throw new DusrelError(
      "INVALID_TYPE",
      "Bulk selections must be an array of page-number arrays.",
      "Bulk selections page-number arrays ki list honi chahiye.",
    );
  }
  const parsed = selections.map((s) => assertPageList(s, "Bulk selection"));
  const json = call(() =>
    backendImpl().bulk_extract(bytes, parsed, toCommon(options)),
  );
  const items = JSON.parse(json) as Array<{
    index: number;
    pdf?: string | null;
    error_code?: string | null;
    error?: string | null;
  }>;
  return items.map((item) => {
    const out: BulkExtractItem = { index: item.index };
    if (typeof item.pdf === "string") out.pdf = base64ToBytes(item.pdf);
    if (typeof item.error_code === "string") out.errorCode = item.error_code;
    if (typeof item.error === "string") out.error = item.error;
    return out;
  });
}
