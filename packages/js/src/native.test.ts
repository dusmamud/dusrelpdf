/**
 * dusrelpdf JS tests — native napi-rs backend.
 *
 * The native-path tests run the same operations through the `.node`
 * addon when it is built (`npm run build` in
 * `crates/dusrelpdf-napi`); they skip when no `.node` is present, so
 * this file also passes on a WASM-only checkout. The fallback tests
 * force the WASM path with `DUSREL_PDF_FORCE_WASM=1` and a fresh
 * module import, so they run whether or not the addon exists.
 *
 * Fixtures come from `makeTestPdf` (the Rust core's own fixture
 * builder, exposed by both backends), so both backends test the
 * same bytes; page counts go through `getPdfInfo`, content checks
 * read the output bytes with `compress: false` where the check
 * needs a content stream.
 */
import * as fs from "node:fs";
import * as path from "node:path";
import { describe, expect, it, vi } from "vitest";
import {
  DusrelError,
  addPageNumbers,
  addWatermark,
  backend,
  bulkExtract,
  extractPages,
  extractPagesRange,
  getPdfInfo,
  imagesToPdf,
  makeTestPdf,
  mergePdfs,
  placeSignature,
  removePages,
  reorderPages,
  rotatePages,
  splitEachPage,
  splitPdf,
  splitPdfRanges,
  version,
} from "./index";

// 1x1 opaque red PNG (imagesToPdf / placeSignature fixture).
const RED_PNG_B64 =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

const redPng = (): Uint8Array =>
  new Uint8Array(Buffer.from(RED_PNG_B64, "base64"));

const pageCount = (pdf: Uint8Array): number => getPdfInfo(pdf).pageCount;

/** The `.node` file the loader looks for first, if it was built. */
function nativeNodePath(): string {
  const arch = process.arch;
  const triple =
    process.platform === "linux"
      ? `linux-${arch}-gnu`
      : process.platform === "win32"
        ? `win32-${arch}-msvc`
        : `${process.platform}-${arch}`;
  return path.resolve(
    process.cwd(),
    "..",
    "..",
    "crates",
    "dusrelpdf-napi",
    `dusrelpdf-napi.${triple}.node`,
  );
}

const nativePresent =
  fs.existsSync(nativeNodePath()) &&
  process.env.DUSREL_PDF_FORCE_WASM !== "1";
const nativeIt = nativePresent ? it : it.skip;

describe("native backend (when the .node is built)", () => {
  nativeIt("reports backend() === 'native'", () => {
    expect(backend()).toBe("native");
  });

  nativeIt("reports the crate version", () => {
    expect(version()).toBe("0.1.0");
  });

  nativeIt("merges, splits and extracts with real fixture PDFs", () => {
    const a = makeTestPdf(2);
    const b = makeTestPdf(3);
    const merged = mergePdfs([a, b]);
    expect(pageCount(merged)).toBe(5);

    const parts = splitPdf(merged, [
      [1, 2],
      [3, 4, 5],
    ]);
    expect(parts).toHaveLength(2);
    expect(pageCount(parts[0])).toBe(2);
    expect(pageCount(parts[1])).toBe(3);

    const ranged = splitPdfRanges(merged, "1-2,5");
    expect(ranged).toHaveLength(2);
    expect(pageCount(ranged[1])).toBe(1);

    expect(splitEachPage(merged)).toHaveLength(5);

    expect(pageCount(extractPages(merged, [5, 1]))).toBe(2);
    expect(pageCount(extractPagesRange(merged, "2-3"))).toBe(2);
  });

  nativeIt("removes, reorders and rotates pages", () => {
    const doc = makeTestPdf(4);
    expect(pageCount(removePages(doc, [2]))).toBe(3);
    expect(pageCount(reorderPages(doc, [4, 3, 2, 1]))).toBe(4);

    const rotated = rotatePages(doc, { angle: 90, compress: false });
    expect(pageCount(rotated)).toBe(4);
    expect(Buffer.from(rotated).toString("latin1")).toContain("/Rotate 90");

    const neg = rotatePages(doc, { angle: -90, compress: false });
    expect(Buffer.from(neg).toString("latin1")).toContain("/Rotate 270");
  });

  nativeIt("stamps watermarks, page numbers, signatures and images", () => {
    const doc = makeTestPdf(2);
    expect(pageCount(addWatermark(doc))).toBe(2);
    expect(pageCount(addPageNumbers(doc, { format: "Page {n}" }))).toBe(2);
    expect(pageCount(placeSignature(doc, redPng()))).toBe(2);
    expect(pageCount(imagesToPdf([redPng()]))).toBe(1);
  });

  nativeIt("reads PDF info", () => {
    const info = getPdfInfo(makeTestPdf(3));
    expect(info.pageCount).toBe(3);
    expect(info.version).toBe("1.7");
    expect(info.encrypted).toBe(false);
    expect(info.pageSizes).toHaveLength(3);
  });

  nativeIt("bulkExtract keeps per-item errors", () => {
    const items = bulkExtract(makeTestPdf(3), [[1], [9], [2, 3]]);
    expect(items).toHaveLength(3);
    expect(items[0].pdf).toBeInstanceOf(Uint8Array);
    expect(items[1].errorCode).toBe("PAGE_OUT_OF_RANGE");
    expect(items[1].pdf).toBeUndefined();
    expect(items[2].pdf).toBeInstanceOf(Uint8Array);
  });

  nativeIt("throws DusrelError with the same codes as WASM", () => {
    const doc = makeTestPdf(3);
    expect(() => extractPages(doc, [9])).toThrowError(DusrelError);
    try {
      extractPages(doc, [9]);
      expect.unreachable();
    } catch (e) {
      expect(e).toBeInstanceOf(DusrelError);
      expect((e as DusrelError).code).toBe("PAGE_OUT_OF_RANGE");
    }
    expect(() => extractPagesRange(doc, "9")).toThrowError(/INVALID_RANGE/);
    expect(() => mergePdfs([])).toThrowError(/EMPTY_INPUT/);
    expect(() => rotatePages(doc, { angle: 45 })).toThrowError(/INVALID_ANGLE/);
    expect(() => extractPages(new Uint8Array([1, 2, 3]), [1])).toThrowError(
      /INVALID_PDF/,
    );
  });
});

describe("WASM fallback", () => {
  it("forces the WASM path with DUSREL_PDF_FORCE_WASM=1 and still merges", async () => {
    const prev = process.env.DUSREL_PDF_FORCE_WASM;
    process.env.DUSREL_PDF_FORCE_WASM = "1";
    try {
      vi.resetModules();
      const forced = await import("./index");
      expect(forced.backend()).toBe("wasm");
      const merged = forced.mergePdfs([
        forced.makeTestPdf(2),
        forced.makeTestPdf(1),
      ]);
      expect(forced.getPdfInfo(merged).pageCount).toBe(3);
      // Errors keep their codes on the fallback path too.
      expect(() => forced.extractPages(merged, [9])).toThrowError(
        /PAGE_OUT_OF_RANGE/,
      );
    } finally {
      if (prev === undefined) delete process.env.DUSREL_PDF_FORCE_WASM;
      else process.env.DUSREL_PDF_FORCE_WASM = prev;
      vi.resetModules();
    }
  });

  it("picks the native addon again on a fresh import once the flag is cleared", async () => {
    if (!nativePresent) {
      expect(backend()).toBe("wasm");
      return;
    }
    delete process.env.DUSREL_PDF_FORCE_WASM;
    vi.resetModules();
    try {
      const fresh = await import("./index");
      expect(fresh.backend()).toBe("native");
    } finally {
      vi.resetModules();
    }
  });
});
