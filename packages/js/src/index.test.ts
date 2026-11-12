/**
 * dusrelpdf JS tests — wrapper over the Rust core.
 *
 * Every test runs through the real backend selected by `./index`:
 * the native `.node` addon when it is built, else the WASM build
 * (`npm run build` via `pretest`). The suite is backend-agnostic on
 * purpose — both backends must behave identically — while
 * `native.test.ts` pins each backend explicitly (native path, plus
 * the `DUSREL_PDF_FORCE_WASM=1` fallback). So these assert the whole
 * chain: TS validation -> camelCase-to-snake_case translation ->
 * backend -> Rust core.
 * Page-count assertions go through `getPdfInfo` (the backend round-trip);
 * content assertions (page order, /Rotate, stamp text, /SMask) read the
 * output bytes with `compress: false` where the check needs a content
 * stream, or straight from the raw bytes where the check is a dict
 * entry (/Rotate, /SMask) that compression never hides.
 *
 * Fixtures: the PNG/JPEG base64 below were generated with Python PIL
 * during development (a 200x80 transparent PNG with a dark stroke, a
 * 120x90 gradient JPEG, a 10x10 opaque red PNG) — the same shapes as
 * the Rust fixtures in crates/dusrelpdf/src/fixtures.rs.
 */
import { describe, expect, it } from "vitest";
import {
  DusrelError,
  addPageNumbers,
  addWatermark,
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
  type BulkExtractItem,
} from "./index";
import {
  mergeOptionsSchema,
  pageSizeSchema,
  pdfBytesSchema,
  positionSchema,
  watermarkOptionsSchema,
} from "./zod";

// 200x80 transparent PNG, dark diagonal stroke + underline.
const SIGNATURE_PNG_B64 =
  "iVBORw0KGgoAAAANSUhEUgAAAMgAAABQCAYAAABcbTqwAAACLklEQVR4nO3dTW7jMAwGUHfO4PufMHfoLILBpEGS+keWRPK9TTcFYn0UHao1kmUBAAAAAAAAoL11Xb8ffxLH1+gLyOxdQ9xuN7kHoVAX2fJuoVHmp0CN7R2jNMncFKehLSPVq9/RJPNSmAb2njWcTeJQkBM+jVO/bXZNEoNiHNRqgxu55qYQO11x5/duMq8/oy8gkqs28t6zCv24Q23Q8w5v5JqL4D84cwi/4nU1Sn8Cf2P0Jh39+twJ+8lsG9PINZagH8zWHP/Mel0VCHiJsQEjXGNGpcMddQg/w8jVV9lgI9+RI197NOUCzbK5sqxjdqXCzLipjFzXKhFkxsZ4lH19I6UOMOIh/ChNco204VXdMEauttIFV7UxHsmgnVSPu9sYdx6fbyfFxtEY7xm5zgkdVKVD+BluIMeFDUjR95HXMeHCUehzjFz7hApGc7Qhx+1CBKKg7cl0m6nDcAi/npHrs2mDcIfrR9bvTReAYo0h99emWrwijWfk+mmKhWuMuajHf1N+Wsey1CzGTDTJ3bDFKkAM1Ueu7gvVGPFUrlnXx90rBx1Z5cfnfc4su1QbuS5dmEN4TpVueD7Cn0Oq1LfLF8AsS77guMs+cjVdiOaoKXPdmywgc0Bsk3UPnLp4h3CePe6JDHvAVxfT3Lqu31n2we5FaAwq2fWfdM1BNZs2tsagqk3vIK8aQXPAkwoPpwEAAJfq9rAi9NLyD0ipvkAHAAAAAAAAAAAAAEL5C97HhYZBsFRgAAAAAElFTkSuQmCC";
// 120x90 gradient JPEG (opaque, no alpha).
const PHOTO_JPEG_B64 =
  "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";
// 10x10 opaque red PNG.
const RED_PNG_B64 =
  "iVBORw0KGgoAAAANSUhEUgAAAAoAAAAKCAIAAAACUFjqAAAAEklEQVR4nGP8z4APMOGVHbHSAEEsAROxCnMTAAAAAElFTkSuQmCC";

const signaturePng = (): Uint8Array =>
  new Uint8Array(Buffer.from(SIGNATURE_PNG_B64, "base64"));
const photoJpeg = (): Uint8Array =>
  new Uint8Array(Buffer.from(PHOTO_JPEG_B64, "base64"));
const redPng = (): Uint8Array =>
  new Uint8Array(Buffer.from(RED_PNG_B64, "base64"));
const garbage = (): Uint8Array =>
  new Uint8Array(Buffer.from("this is not a pdf at all", "utf8"));

const pageCount = (pdf: Uint8Array): number => getPdfInfo(pdf).pageCount;
const raw = (pdf: Uint8Array): string => Buffer.from(pdf).toString("latin1");

/**
 * Page labels in document order, by following the Pages `/Kids` array
 * to each page dict's `/Contents` stream ("Page N" text). Byte position
 * in the file does NOT give page order — objects are stored in
 * insertion order — so indexOf("(Page 3)") checks are wrong; this
 * helper is the correct way to verify extract/reorder order.
 */
function pageLabels(pdf: Uint8Array): string[] {
  const text = raw(pdf);
  const kids = /\/Kids\s*\[([^\]]*)\]/.exec(text);
  if (!kids) throw new Error("no /Kids array in output");
  const ids = [...kids[1].matchAll(/(\d+)\s+0\s+R/g)].map((m) => m[1]);
  return ids.map((id) => {
    const obj = new RegExp(`(?:^|\\n)${id} 0 obj([\\s\\S]*?)endobj`, "m").exec(text);
    if (!obj) throw new Error(`page object ${id} not found`);
    const contents = /\/Contents\s+(\d+)\s+0\s+R/.exec(obj[1]);
    if (!contents) throw new Error(`no /Contents ref on page ${id}`);
    const stream = new RegExp(
      `(?:^|\\n)${contents[1]} 0 obj([\\s\\S]*?)endstream`,
      "m",
    ).exec(text);
    if (!stream) throw new Error(`content stream ${contents[1]} not found`);
    const label = /\(Page (\d+)\)/.exec(stream[1]);
    if (!label) throw new Error("no Page label in stream");
    return `Page ${label[1]}`;
  });
}

function expectDusrel(fn: () => unknown, code: string): DusrelError {
  try {
    fn();
  } catch (e) {
    expect(e).toBeInstanceOf(DusrelError);
    const err = e as DusrelError;
    expect(err.code).toBe(code);
    expect(err.reasonEn.length).toBeGreaterThan(0);
    expect(err.reasonHi.length).toBeGreaterThan(0);
    expect(err.message).toContain(code);
    return err;
  }
  throw new Error(`Expected DusrelError ${code}, but nothing was thrown`);
}

describe("version + fixtures", () => {
  it("version() is 0.1.0", () => {
    expect(version()).toBe("0.1.0");
  });

  it("makeTestPdf builds N pages", () => {
    expect(pageCount(makeTestPdf(1))).toBe(1);
    expect(pageCount(makeTestPdf(5))).toBe(5);
  });

  it("makeTestPdf(0) clamps to 1 page (Rust core)", () => {
    expect(pageCount(makeTestPdf(0))).toBe(1);
  });

  it("makeTestPdf rejects bad counts", () => {
    expectDusrel(() => makeTestPdf(-1), "INVALID_ARGUMENT");
    expectDusrel(() => makeTestPdf(2.5), "INVALID_ARGUMENT");
    expectDusrel(() => makeTestPdf("3" as unknown as number), "INVALID_ARGUMENT");
  });

  it("getPdfInfo shape: count, version, encrypted, page sizes", () => {
    const info = getPdfInfo(makeTestPdf(3));
    expect(info.pageCount).toBe(3);
    expect(info.version).toBe("1.7");
    expect(info.encrypted).toBe(false);
    expect(info.title).toBeUndefined();
    expect(info.pageSizes).toEqual([
      [595, 842],
      [595, 842],
      [595, 842],
    ]);
  });
});

describe("mergePdfs", () => {
  it("merges 2 + 3 pages into 5", () => {
    const out = mergePdfs([makeTestPdf(2), makeTestPdf(3)]);
    expect(pageCount(out)).toBe(5);
  });

  it("merges a single input", () => {
    expect(pageCount(mergePdfs([makeTestPdf(4)]))).toBe(4);
  });

  it("accepts Node Buffers (Buffer is a Uint8Array)", () => {
    const out = mergePdfs([Buffer.from(makeTestPdf(1)), Buffer.from(makeTestPdf(1))] as unknown as Uint8Array[]);
    expect(pageCount(out)).toBe(2);
  });

  it("empty input list is EMPTY_INPUT", () => {
    expectDusrel(() => mergePdfs([]), "EMPTY_INPUT");
  });

  it("non-Uint8Array inputs are INVALID_TYPE, never TypeError", () => {
    expectDusrel(() => mergePdfs("not bytes" as unknown as Uint8Array[]), "INVALID_TYPE");
    expectDusrel(() => mergePdfs([makeTestPdf(1), "x" as unknown as Uint8Array]), "INVALID_TYPE");
    expectDusrel(() => mergePdfs(null as unknown as Uint8Array[]), "INVALID_TYPE");
  });

  it("garbage bytes are INVALID_PDF", () => {
    expectDusrel(() => mergePdfs([garbage()]), "INVALID_PDF");
  });
});

describe("split", () => {
  it("splitPdf splits by page lists", () => {
    const outs = splitPdf(makeTestPdf(3), [[1, 2], [3]]);
    expect(outs).toHaveLength(2);
    expect(pageCount(outs[0])).toBe(2);
    expect(pageCount(outs[1])).toBe(1);
  });

  it("splitPdfRanges splits by range string", () => {
    const outs = splitPdfRanges(makeTestPdf(5), "1-2,3,4-5");
    expect(outs).toHaveLength(3);
    expect(outs.map(pageCount)).toEqual([2, 1, 2]);
  });

  it("splitEachPage gives one PDF per page", () => {
    const outs = splitEachPage(makeTestPdf(4));
    expect(outs).toHaveLength(4);
    expect(outs.every((o) => pageCount(o) === 1)).toBe(true);
  });

  it("splitPdf rejects bad ranges", () => {
    const pdf = makeTestPdf(3);
    expectDusrel(() => splitPdf(pdf, []), "EMPTY_INPUT");
    expectDusrel(() => splitPdf(pdf, "1-2" as unknown as number[][]), "INVALID_TYPE");
    expectDusrel(() => splitPdf(pdf, [[1.5]]), "INVALID_ARGUMENT");
    expectDusrel(() => splitPdf(pdf, [[99]]), "PAGE_OUT_OF_RANGE");
    expectDusrel(() => splitPdf(pdf, [[0]]), "PAGE_OUT_OF_RANGE");
  });

  it("splitPdfRanges rejects bad range strings", () => {
    const pdf = makeTestPdf(3);
    expectDusrel(() => splitPdfRanges(pdf, ""), "INVALID_RANGE");
    expectDusrel(() => splitPdfRanges(pdf, "abc"), "INVALID_RANGE");
    expectDusrel(() => splitPdfRanges(pdf, "5-2"), "INVALID_RANGE");
    expectDusrel(() => splitPdfRanges(pdf, "1-9"), "INVALID_RANGE");
    expectDusrel(() => splitPdfRanges(pdf, 123 as unknown as string), "INVALID_TYPE");
  });
});

describe("extract / remove / reorder", () => {
  it("extractPages keeps the given order (incl. repeats)", () => {
    const out = extractPages(makeTestPdf(3), [3, 1, 3], { compress: false });
    expect(pageCount(out)).toBe(3);
    expect(pageLabels(out)).toEqual(["Page 3", "Page 1", "Page 3"]);
  });

  it("extractPagesRange parses 1-3,5", () => {
    expect(pageCount(extractPagesRange(makeTestPdf(5), "1-3,5"))).toBe(4);
    expect(pageCount(extractPagesRange(makeTestPdf(5), "2"))).toBe(1);
  });

  it("extract rejects empty / out-of-range / malformed input", () => {
    const pdf = makeTestPdf(3);
    expectDusrel(() => extractPages(pdf, []), "EMPTY_INPUT");
    expectDusrel(() => extractPages(pdf, [99]), "PAGE_OUT_OF_RANGE");
    expectDusrel(() => extractPages(pdf, [0]), "PAGE_OUT_OF_RANGE");
    expectDusrel(() => extractPages(pdf, "1" as unknown as number[]), "INVALID_TYPE");
    expectDusrel(() => extractPagesRange(pdf, ""), "INVALID_RANGE");
    expectDusrel(() => extractPagesRange(pdf, "0"), "INVALID_RANGE");
  });

  it("removePages deletes pages; empty list is a no-op", () => {
    const pdf = makeTestPdf(3);
    expect(pageCount(removePages(pdf, [2]))).toBe(2);
    expect(pageCount(removePages(pdf, [1, 3]))).toBe(1);
    const untouched = removePages(pdf, []);
    expect(Buffer.from(untouched).equals(Buffer.from(pdf))).toBe(true);
    expectDusrel(() => removePages(pdf, [99]), "PAGE_OUT_OF_RANGE");
  });

  it("reorderPages reverses the order (via the /Kids array)", () => {
    const out = reorderPages(makeTestPdf(3), [3, 2, 1], { compress: false });
    expect(pageLabels(out)).toEqual(["Page 3", "Page 2", "Page 1"]);
  });

  it("reorderPages rejects non-permutations", () => {
    const pdf = makeTestPdf(3);
    expectDusrel(() => reorderPages(pdf, [1, 2]), "INVALID_ARGUMENT");
    expectDusrel(() => reorderPages(pdf, [1, 1, 2]), "INVALID_ARGUMENT");
    // An out-of-range entry is PAGE_OUT_OF_RANGE (checked before the
    // duplicate check in the core's validate_permutation).
    expectDusrel(() => reorderPages(pdf, [1, 2, 4]), "PAGE_OUT_OF_RANGE");
  });
});

describe("rotatePages", () => {
  it("rotates all pages 90 by default", () => {
    const out = rotatePages(makeTestPdf(2));
    expect(pageCount(out)).toBe(2);
    expect(raw(out)).toContain("/Rotate 90");
  });

  it("rotates 180 on one page only", () => {
    const out = rotatePages(makeTestPdf(3), { angle: 180, pages: [1] });
    expect(pageCount(out)).toBe(3);
    expect(raw(out)).toContain("/Rotate 180");
  });

  it("normalises -90 to 270", () => {
    const out = rotatePages(makeTestPdf(1), { angle: -90 });
    expect(raw(out)).toContain("/Rotate 270");
  });

  it("rejects angles that are not multiples of 90", () => {
    const pdf = makeTestPdf(2);
    expectDusrel(() => rotatePages(pdf, { angle: 45 }), "INVALID_ANGLE");
    expectDusrel(() => rotatePages(pdf, { angle: 90.5 }), "INVALID_ANGLE");
    expectDusrel(() => rotatePages(pdf, { angle: 100 }), "INVALID_ANGLE");
  });
});

describe("imagesToPdf", () => {
  it("one page per image, A4 by default", () => {
    const out = imagesToPdf([signaturePng(), photoJpeg()]);
    const info = getPdfInfo(out);
    expect(info.pageCount).toBe(2);
    expect(info.pageSizes).toEqual([
      [595, 842],
      [595, 842],
    ]);
  });

  it("accepts a red PNG too", () => {
    expect(pageCount(imagesToPdf([redPng()]))).toBe(1);
  });

  it("pageSize auto sizes the page to the image", () => {
    const info = getPdfInfo(imagesToPdf([redPng()], { pageSize: "auto" }));
    expect(info.pageSizes).toEqual([[10, 10]]);
  });

  it("pageSize letter + fill", () => {
    const info = getPdfInfo(
      imagesToPdf([redPng()], { pageSize: "letter", fit: "fill" }),
    );
    expect(info.pageSizes).toEqual([[612, 792]]);
  });

  it("custom page size { width, height }", () => {
    const info = getPdfInfo(
      imagesToPdf([redPng()], { pageSize: { width: 200, height: 200 } }),
    );
    expect(info.pageSizes).toEqual([[200, 200]]);
  });

  it("rejects empty / bad images", () => {
    expectDusrel(() => imagesToPdf([]), "EMPTY_INPUT");
    expectDusrel(() => imagesToPdf([garbage()]), "INVALID_IMAGE");
    expectDusrel(
      () => imagesToPdf(["x" as unknown as Uint8Array]),
      "INVALID_TYPE",
    );
    expectDusrel(
      () => imagesToPdf([redPng()], { pageSize: "tabloid" as never }),
      "INVALID_ARGUMENT",
    );
  });
});

describe("addWatermark", () => {
  it("defaults to DRAFT text", () => {
    const out = addWatermark(makeTestPdf(2));
    expect(pageCount(out)).toBe(2);
  });

  it("stamps custom text (visible in the raw content stream)", () => {
    const out = addWatermark(makeTestPdf(3), {
      text: "CONFIDENTIAL",
      rotation: 0,
      compress: false,
    });
    expect(raw(out)).toContain("CONFIDENTIAL");
  });

  it("stamps on selected pages only", () => {
    const out = addWatermark(makeTestPdf(3), {
      text: "ONLYPAGE2",
      rotation: 0,
      pages: [2],
      compress: false,
    });
    // The stamp text appears once (page 2), not three times.
    expect(raw(out).split("ONLYPAGE2").length - 1).toBe(1);
  });

  it("stamps an image watermark (number-array path to wasm)", () => {
    const out = addWatermark(makeTestPdf(1), {
      image: signaturePng(),
      rotation: 0,
      opacity: 0.5,
    });
    expect(pageCount(out)).toBe(1);
    expect(raw(out)).toContain("SMask");
  });

  it("rejects text + image together, bad opacity, empty text", () => {
    const pdf = makeTestPdf(1);
    expectDusrel(
      () => addWatermark(pdf, { text: "X", image: signaturePng() }),
      "INVALID_ARGUMENT",
    );
    expectDusrel(() => addWatermark(pdf, { opacity: 2 }), "INVALID_ARGUMENT");
    expectDusrel(() => addWatermark(pdf, { opacity: -0.1 }), "INVALID_ARGUMENT");
    expectDusrel(() => addWatermark(pdf, { text: "" }), "INVALID_ARGUMENT");
    expectDusrel(
      () => addWatermark(pdf, { image: "x" as unknown as Uint8Array }),
      "INVALID_TYPE",
    );
    expectDusrel(() => addWatermark(pdf, { pages: [99] }), "PAGE_OUT_OF_RANGE");
  });
});

describe("addPageNumbers", () => {
  it("stamps the default {n} format", () => {
    expect(pageCount(addPageNumbers(makeTestPdf(3)))).toBe(3);
  });

  it("stamps Page {n} of {total} (visible in raw content)", () => {
    const out = addPageNumbers(makeTestPdf(3), {
      format: "Page {n} of {total}",
      compress: false,
    });
    expect(raw(out)).toContain("Page 1 of 3");
    expect(raw(out)).toContain("Page 3 of 3");
  });

  it("startAt shifts the printed numbers", () => {
    const out = addPageNumbers(makeTestPdf(2), {
      format: "Page {n}",
      startAt: 10,
      compress: false,
    });
    expect(raw(out)).toContain("Page 10");
    expect(raw(out)).toContain("Page 11");
  });

  it("rejects a format without {n}", () => {
    expectDusrel(
      () => addPageNumbers(makeTestPdf(1), { format: "Page" }),
      "INVALID_ARGUMENT",
    );
  });
});

describe("placeSignature", () => {
  it("places the signature image; transparent PNG keeps /SMask", () => {
    const out = placeSignature(makeTestPdf(1), signaturePng());
    expect(pageCount(out)).toBe(1);
    // Deep SMask behaviour is covered by the Rust tests; here we assert
    // the output loads as 1 page AND carries the soft-mask entry, which
    // is what stops the signature rendering as an opaque rectangle.
    expect(raw(out)).toContain("SMask");
  });

  it("places on a chosen page with a chosen size", () => {
    const out = placeSignature(makeTestPdf(3), signaturePng(), {
      page: 3,
      x: 100,
      y: 100,
      width: 72,
    });
    expect(pageCount(out)).toBe(3);
  });

  it("rejects bad page / bad image", () => {
    const pdf = makeTestPdf(2);
    expectDusrel(() => placeSignature(pdf, signaturePng(), { page: 99 }), "PAGE_OUT_OF_RANGE");
    expectDusrel(() => placeSignature(pdf, signaturePng(), { page: 0 }), "PAGE_OUT_OF_RANGE");
    expectDusrel(() => placeSignature(pdf, garbage()), "INVALID_IMAGE");
    expectDusrel(
      () => placeSignature(pdf, "sig" as unknown as Uint8Array),
      "INVALID_TYPE",
    );
  });
});

describe("getPdfInfo + errors", () => {
  it("rejects garbage and empty bytes", () => {
    expectDusrel(() => getPdfInfo(garbage()), "INVALID_PDF");
    expectDusrel(() => getPdfInfo(new Uint8Array(0)), "EMPTY_INPUT");
  });

  it("rejects non-Uint8Array input with INVALID_TYPE everywhere", () => {
    expectDusrel(() => getPdfInfo(123 as unknown as Uint8Array), "INVALID_TYPE");
    expectDusrel(() => getPdfInfo(null as unknown as Uint8Array), "INVALID_TYPE");
    expectDusrel(() => getPdfInfo("pdf" as unknown as Uint8Array), "INVALID_TYPE");
    expectDusrel(() => extractPages("x" as unknown as Uint8Array, [1]), "INVALID_TYPE");
    expectDusrel(() => rotatePages({} as unknown as Uint8Array), "INVALID_TYPE");
    expectDusrel(() => addWatermark(undefined as unknown as Uint8Array), "INVALID_TYPE");
    expectDusrel(() => addPageNumbers(42 as unknown as Uint8Array), "INVALID_TYPE");
    expectDusrel(() => splitEachPage([] as unknown as Uint8Array), "INVALID_TYPE");
    expectDusrel(() => removePages("x" as unknown as Uint8Array, [1]), "INVALID_TYPE");
    expectDusrel(() => bulkExtract("x" as unknown as Uint8Array, [[1]]), "INVALID_TYPE");
  });
});

describe("bulkExtract", () => {
  it("extracts many selections; one bad selection fails only its item", () => {
    const items: BulkExtractItem[] = bulkExtract(makeTestPdf(3), [[1, 2], [99], [3]]);
    expect(items).toHaveLength(3);
    expect(items[0].index).toBe(0);
    expect(items[0].pdf).toBeInstanceOf(Uint8Array);
    expect(pageCount(items[0].pdf as Uint8Array)).toBe(2);
    expect(items[0].errorCode).toBeUndefined();
    expect(items[1].pdf).toBeUndefined();
    expect(items[1].errorCode).toBe("PAGE_OUT_OF_RANGE");
    expect(typeof items[1].error).toBe("string");
    expect(items[2].pdf).toBeInstanceOf(Uint8Array);
    expect(pageCount(items[2].pdf as Uint8Array)).toBe(1);
  });

  it("an empty selection fails its own item (EMPTY_INPUT)", () => {
    const items = bulkExtract(makeTestPdf(2), [[1], []]);
    expect(items[0].errorCode).toBeUndefined();
    expect(items[1].errorCode).toBe("EMPTY_INPUT");
  });
});

describe("zod schemas", () => {
  it("accepts valid options", () => {
    expect(mergeOptionsSchema.parse({ preserveMetadata: false }).preserveMetadata).toBe(false);
    expect(pageSizeSchema.parse({ width: 200, height: 100 })).toEqual({
      width: 200,
      height: 100,
    });
    expect(pageSizeSchema.parse("a4")).toBe("a4");
  });

  it("rejects bad shapes", () => {
    expect(() => pdfBytesSchema.parse("x")).toThrow();
    expect(() => positionSchema.parse("middle")).toThrow();
    expect(() => watermarkOptionsSchema.parse({ opacity: "high" })).toThrow();
    expect(() => watermarkOptionsSchema.parse({ image: "x" })).toThrow();
  });
});
