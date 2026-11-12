/**
 * dusrelpdf — Zod schemas.
 *
 * Optional integration: `zod` is a peer dependency (optional).
 * Importing this module requires zod to be installed; the main entry
 * point (`index.ts`) never imports zod.
 *
 * The schemas mirror the TS option interfaces in `index.ts` (which use
 * camelCase keys; the wrapper translates them to the Rust snake_case
 * JSON keys). They check shape only — page existence, PDF validity
 * and value ranges like opacity 0–1 are enforced by the core and
 * reported as `DusrelError`s, not Zod errors.
 */
import { z } from "zod";

export const pdfBytesSchema = z.instanceof(Uint8Array, {
  message: "Input must be a Uint8Array. / Input Uint8Array hona chahiye.",
});

export const pdfArraySchema = z.array(pdfBytesSchema);
export const imageArraySchema = z.array(pdfBytesSchema);

export const pageListSchema = z.array(
  z.number().int("Page numbers must be integers. / Page numbers integer hone chahiye."),
);

export const rangeStringSchema = z
  .string()
  .min(1, 'Page range must not be empty, e.g. "1-3,5". / Page range khaali nahi hona chahiye, jaise "1-3,5".');

export const positionSchema = z.enum([
  "top_left",
  "top_center",
  "top_right",
  "bottom_left",
  "bottom_center",
  "bottom_right",
  "center",
]);

export const baseFontSchema = z.enum([
  "helvetica",
  "helvetica_bold",
  "times_roman",
  "courier",
  "courier_bold",
]);

export const imageFitSchema = z.enum(["contain", "fill"]);

export const pageSizeSchema = z.union([
  z.enum(["auto", "a4", "letter"]),
  z.object({
    width: z.number().positive("Custom width must be positive. / Custom width positive hona chahiye."),
    height: z.number().positive("Custom height must be positive. / Custom height positive hona chahiye."),
  }),
]);

export const commonOptionsSchema = z.object({
  password: z.string().optional(),
  compress: z.boolean().optional(),
});

export const mergeOptionsSchema = commonOptionsSchema.extend({
  preserveMetadata: z.boolean().optional(),
  addBookmarks: z.boolean().optional(),
});

export const pageSetOptionsSchema = commonOptionsSchema;

export const rotateOptionsSchema = commonOptionsSchema.extend({
  angle: z.number().int().optional(),
  pages: pageListSchema.optional(),
});

export const imagesToPdfOptionsSchema = z.object({
  pageSize: pageSizeSchema.optional(),
  fit: imageFitSchema.optional(),
  background: z.tuple([z.number(), z.number(), z.number()]).optional(),
});

export const watermarkOptionsSchema = commonOptionsSchema.extend({
  text: z.string().optional(),
  image: pdfBytesSchema.optional(),
  opacity: z.number().optional(),
  rotation: z.number().optional(),
  fontSize: z.number().optional(),
  font: baseFontSchema.optional(),
  position: positionSchema.optional(),
  x: z.number().optional(),
  y: z.number().optional(),
  pages: pageListSchema.optional(),
  imageWidth: z.number().optional(),
  imageHeight: z.number().optional(),
});

export const pageNumberOptionsSchema = commonOptionsSchema.extend({
  format: z.string().optional(),
  position: positionSchema.optional(),
  startAt: z.number().int().optional(),
  fontSize: z.number().optional(),
  font: baseFontSchema.optional(),
  margin: z.number().optional(),
  pages: pageListSchema.optional(),
});

export const signatureOptionsSchema = commonOptionsSchema.extend({
  page: z.number().int().optional(),
  x: z.number().optional(),
  y: z.number().optional(),
  width: z.number().optional(),
  height: z.number().optional(),
});

export const pdfInfoSchema = z.object({
  pageCount: z.number().int(),
  version: z.string(),
  encrypted: z.boolean(),
  title: z.string().optional(),
  pageSizes: z.array(z.tuple([z.number(), z.number()])),
});

export type Position = z.infer<typeof positionSchema>;
export type BaseFont = z.infer<typeof baseFontSchema>;
export type ImageFit = z.infer<typeof imageFitSchema>;
export type PageSize = z.infer<typeof pageSizeSchema>;
export type CommonOptions = z.infer<typeof commonOptionsSchema>;
export type MergeOptions = z.infer<typeof mergeOptionsSchema>;
export type PageSetOptions = z.infer<typeof pageSetOptionsSchema>;
export type RotateOptions = z.infer<typeof rotateOptionsSchema>;
export type ImagesToPdfOptions = z.infer<typeof imagesToPdfOptionsSchema>;
export type WatermarkOptions = z.infer<typeof watermarkOptionsSchema>;
export type PageNumberOptions = z.infer<typeof pageNumberOptionsSchema>;
export type SignatureOptions = z.infer<typeof signatureOptionsSchema>;
export type PdfInfo = z.infer<typeof pdfInfoSchema>;
