// Package dusrelpdf is a Go (cgo) wrapper over the dusrelpdf Rust core,
// via the C ABI in crates/dusrelpdf-ffi. All PDF logic lives in Rust;
// this package only converts Go values to C buffers / JSON options and
// back. Bytes in, bytes out, never a panic on bad input.
//
// Build the shared library first (see README.md):
//
//	cargo build -p dusrelpdf-ffi --release
//
// The cdylib (target/release/libdusrelpdf.so on Linux,
// libdusrelpdf.dylib on macOS, dusrelpdf.dll on Windows) is linked
// by cgo and found at runtime through an rpath baked into the
// test/binary — no LD_LIBRARY_PATH needed for the standard layout.
package dusrelpdf

/*
#cgo LDFLAGS: -L${SRCDIR}/../../target/release -ldusrelpdf -Wl,-rpath,${SRCDIR}/../../target/release
#include "../../crates/dusrelpdf-ffi/dusrelpdf.h"
#include <stdlib.h>
*/
import "C"

import (
	"encoding/json"
	"reflect"
	"runtime"
	"unsafe"
)

// Error is a dusrelpdf failure: a stable machine-readable Code plus
// human-readable reasons in English and Hindi. It implements error;
// Error() renders "CODE: reason_en".
type Error struct {
	Code      string `json:"code"`
	MessageEN string `json:"message_en"`
	MessageHI string `json:"message_hi"`
}

func (e *Error) Error() string { return e.Code + ": " + e.MessageEN }

// PdfInfo is the summary returned by GetPDFInfo.
type PdfInfo struct {
	PageCount uint32       `json:"page_count"`
	Version   string       `json:"version"`
	Encrypted bool         `json:"encrypted"`
	Title     *string      `json:"title"`
	PageSizes [][2]float32 `json:"page_sizes"`
}

// BulkItem is one entry of a BulkExtract result. On success PDF holds
// the output bytes; on failure PDF is nil and ErrorCode/Error are set.
type BulkItem struct {
	Index     int    `json:"index"`
	PDF       []byte `json:"pdf"`
	ErrorCode string `json:"error_code"`
	Error     string `json:"error"`
}

// --- Options ---------------------------------------------------------------
//
// Every options struct mirrors the Rust core options (same snake_case
// JSON field names, see crates/dusrelpdf/src/options.rs). All fields
// are pointers with `omitempty`: a nil field is left out of the JSON
// and the Rust default applies. Passing a nil options pointer means
// "all defaults". Use the Default*Options constructors for a struct
// pre-filled with the Rust defaults, then change what you need.

// Helpers to take the address of a literal for an option field.
func Bool(v bool) *bool          { return &v }
func String(v string) *string    { return &v }
func Int(v int) *int             { return &v }
func Uint32(v uint32) *uint32    { return &v }
func Float32(v float32) *float32 { return &v }

// CommonOptions is shared by every operation that reads a PDF. It is
// embedded (flattened) in the other options structs.
type CommonOptions struct {
	// Password for an encrypted PDF. Empty-password PDFs open without it.
	Password *string `json:"password,omitempty"`
	// Compress output streams (default true).
	Compress *bool `json:"compress,omitempty"`
}

// MergeOptions for Merge.
type MergeOptions struct {
	CommonOptions
	// Copy the first input's document info into the output (default true).
	PreserveMetadata *bool `json:"preserve_metadata,omitempty"`
	// Add a flat outline bookmark for the first page of each input (default true).
	AddBookmarks *bool `json:"add_bookmarks,omitempty"`
}

// DefaultMergeOptions returns MergeOptions with the Rust defaults.
func DefaultMergeOptions() *MergeOptions {
	return &MergeOptions{
		CommonOptions:    CommonOptions{Compress: Bool(true)},
		PreserveMetadata: Bool(true),
		AddBookmarks:     Bool(true),
	}
}

// PageSetOptions for split / extract / remove / reorder / bulk extract.
type PageSetOptions struct {
	CommonOptions
}

// DefaultPageSetOptions returns PageSetOptions with the Rust defaults.
func DefaultPageSetOptions() *PageSetOptions {
	return &PageSetOptions{CommonOptions: CommonOptions{Compress: Bool(true)}}
}

// RotateOptions for Rotate.
type RotateOptions struct {
	CommonOptions
	// Rotation in degrees; must be a multiple of 90 (default 90).
	// Negative angles are normalised (e.g. -90 becomes 270).
	Angle *int `json:"angle,omitempty"`
	// Pages to rotate, 1-based. Nil = every page.
	Pages []uint32 `json:"pages,omitempty"`
}

// DefaultRotateOptions returns RotateOptions with the Rust defaults.
func DefaultRotateOptions() *RotateOptions {
	return &RotateOptions{
		CommonOptions: CommonOptions{Compress: Bool(true)},
		Angle:         Int(90),
	}
}

// ImagesToPdfOptions for ImagesToPDF.
type ImagesToPdfOptions struct {
	// Page size: "auto", "a4" (default), "letter", or
	// map[string]any{"width": w, "height": h} for a custom size.
	PageSize any `json:"page_size,omitempty"`
	// Fit mode: "contain" (default) or "fill".
	Fit *string `json:"fit,omitempty"`
	// Background colour as [r, g, b] 0.0-1.0 for letterboxed areas.
	// Default white; nil keeps the default.
	Background []float32 `json:"background,omitempty"`
}

// DefaultImagesToPdfOptions returns ImagesToPdfOptions with the Rust defaults.
func DefaultImagesToPdfOptions() *ImagesToPdfOptions {
	return &ImagesToPdfOptions{
		PageSize:   "a4",
		Fit:        String("contain"),
		Background: []float32{1, 1, 1},
	}
}

// WatermarkOptions for AddWatermark. The image itself is passed as a
// separate argument to AddWatermark (it overrides any text); this
// struct carries only the text form and the shared settings.
type WatermarkOptions struct {
	CommonOptions
	// Watermark text. Mutually exclusive with the image argument.
	Text *string `json:"text,omitempty"`
	// Opacity 0.0-1.0 (default 0.3).
	Opacity *float32 `json:"opacity,omitempty"`
	// Rotation in degrees, clockwise (default 45).
	Rotation *float32 `json:"rotation,omitempty"`
	// Font size in points (text only, default 48).
	FontSize *float32 `json:"font_size,omitempty"`
	// Font: "helvetica" (default), "helvetica_bold", "times_roman",
	// "courier", "courier_bold".
	Font *string `json:"font,omitempty"`
	// Position: "top_left", "top_center", "top_right", "bottom_left",
	// "bottom_center", "bottom_right", "center" (default).
	Position *string `json:"position,omitempty"`
	// Custom position in points from the bottom-left corner.
	X *float32 `json:"x,omitempty"`
	Y *float32 `json:"y,omitempty"`
	// Pages to watermark, 1-based. Nil = every page.
	Pages []uint32 `json:"pages,omitempty"`
	// Image width/height in points (image watermarks only).
	ImageWidth  *float32 `json:"image_width,omitempty"`
	ImageHeight *float32 `json:"image_height,omitempty"`
}

// DefaultWatermarkOptions returns WatermarkOptions with the Rust defaults.
func DefaultWatermarkOptions() *WatermarkOptions {
	return &WatermarkOptions{
		CommonOptions: CommonOptions{Compress: Bool(true)},
		Text:          String("DRAFT"),
		Opacity:       Float32(0.3),
		Rotation:      Float32(45),
		FontSize:      Float32(48),
		Font:          String("helvetica"),
		Position:      String("center"),
	}
}

// PageNumberOptions for AddPageNumbers.
type PageNumberOptions struct {
	CommonOptions
	// Template with {n} for the page number and {total} for the total,
	// e.g. "{n}", "Page {n} of {total}". Default "{n}".
	Format *string `json:"format,omitempty"`
	// Position (default "bottom_center").
	Position *string `json:"position,omitempty"`
	// First printed number (default 1).
	StartAt *uint32 `json:"start_at,omitempty"`
	// Font size in points (default 10).
	FontSize *float32 `json:"font_size,omitempty"`
	// Font (default "helvetica").
	Font *string `json:"font,omitempty"`
	// Margin from the page edge in points (default 36).
	Margin *float32 `json:"margin,omitempty"`
	// Pages to number, 1-based. Nil = every page.
	Pages []uint32 `json:"pages,omitempty"`
}

// DefaultPageNumberOptions returns PageNumberOptions with the Rust defaults.
func DefaultPageNumberOptions() *PageNumberOptions {
	return &PageNumberOptions{
		CommonOptions: CommonOptions{Compress: Bool(true)},
		Format:        String("{n}"),
		Position:      String("bottom_center"),
		StartAt:       Uint32(1),
		FontSize:      Float32(10),
		Font:          String("helvetica"),
		Margin:        Float32(36),
	}
}

// SignatureOptions for PlaceSignature.
type SignatureOptions struct {
	CommonOptions
	// Target page, 1-based (default 1).
	Page *uint32 `json:"page,omitempty"`
	// Position in points from the bottom-left corner (default 36, 36).
	X *float32 `json:"x,omitempty"`
	Y *float32 `json:"y,omitempty"`
	// Display size in points. Nil width = 144 pt with height from aspect.
	Width  *float32 `json:"width,omitempty"`
	Height *float32 `json:"height,omitempty"`
}

// DefaultSignatureOptions returns SignatureOptions with the Rust defaults.
func DefaultSignatureOptions() *SignatureOptions {
	return &SignatureOptions{
		CommonOptions: CommonOptions{Compress: Bool(true)},
		Page:          Uint32(1),
		X:             Float32(36),
		Y:             Float32(36),
	}
}

// Version returns the library version, e.g. "0.1.0".
func Version() string {
	return C.GoString(C.dusrelpdf_version())
}

// --- internal plumbing -----------------------------------------------------

// cBytes returns a C pointer to b's data (nil for an empty slice).
func cBytes(b []byte) (*C.uchar, C.size_t) {
	if len(b) == 0 {
		return nil, 0
	}
	return (*C.uchar)(unsafe.Pointer(&b[0])), C.size_t(len(b))
}

// cStringOf marshals options to JSON, or returns NULL for nil options
// (all Rust defaults). The returned C string must be freed by the caller.
func cStringOf(v any) *C.char {
	if v == nil {
		return nil
	}
	// A typed nil pointer (e.g. (*MergeOptions)(nil)) is non-nil as an
	// any — check it explicitly, or json.Marshal would send "null"
	// and Rust would reject it as "not an object".
	if rv := reflect.ValueOf(v); rv.Kind() == reflect.Ptr && rv.IsNil() {
		return nil
	}
	data, err := json.Marshal(v)
	if err != nil {
		// Options structs are JSON-safe by construction; this is
		// unreachable in practice, but never pass garbage to C.
		return nil
	}
	return C.CString(string(data))
}

// cStringList marshals a []uint32 to a JSON array C string.
func cStringList(pages []uint32) *C.char {
	data, _ := json.Marshal(pages)
	return C.CString(string(data))
}

// readResult copies a *C.DusrelResult into Go bytes and frees the C side.
// On error it returns a *Error carrying the dusrelpdf error code.
func readResult(r *C.DusrelResult) ([]byte, error) {
	defer C.dusrelpdf_free_result(r)
	if r.ok == 0 {
		var e Error
		if r.error_json != nil {
			_ = json.Unmarshal([]byte(C.GoString(r.error_json)), &e)
		}
		if e.Code == "" {
			e.Code = "OPERATION_FAILED"
		}
		return nil, &e
	}
	if r.data == nil || r.data_len == 0 {
		return []byte{}, nil
	}
	out := C.GoBytes(unsafe.Pointer(r.data), C.int(r.data_len))
	runtime.KeepAlive(r)
	return out, nil
}

// readResultList copies a *C.DusrelResultList into Go and frees the C side.
func readResultList(r *C.DusrelResultList) ([][]byte, error) {
	defer C.dusrelpdf_free_result_list(r)
	if r.ok == 0 {
		var e Error
		if r.error_json != nil {
			_ = json.Unmarshal([]byte(C.GoString(r.error_json)), &e)
		}
		if e.Code == "" {
			e.Code = "OPERATION_FAILED"
		}
		return nil, &e
	}
	if r.items == nil || r.count == 0 {
		return [][]byte{}, nil
	}
	items := unsafe.Slice(r.items, int(r.count))
	out := make([][]byte, 0, len(items))
	for _, item := range items {
		if item.ptr == nil || item.len == 0 {
			out = append(out, []byte{})
			continue
		}
		out = append(out, C.GoBytes(unsafe.Pointer(item.ptr), C.int(item.len)))
	}
	runtime.KeepAlive(r)
	return out, nil
}

// cBytesArray builds a C-allocated array of DusrelBytes for a
// multi-input call. The array lives in C memory (a Go slice of
// structs holding Go pointers cannot cross the cgo boundary), while
// the byte pointers inside it point at the caller's Go slices —
// those must stay alive for the call (KeepAlive at the call site).
// Free the returned pointer with C.free after the call.
func cBytesArray(inputs [][]byte) *C.DusrelBytes {
	if len(inputs) == 0 {
		return nil
	}
	arr := (*C.DusrelBytes)(C.malloc(C.size_t(len(inputs)) * C.size_t(unsafe.Sizeof(C.DusrelBytes{}))))
	slice := unsafe.Slice(arr, len(inputs))
	for i, b := range inputs {
		ptr, ln := cBytes(b)
		slice[i] = C.DusrelBytes{ptr: ptr, len: ln}
	}
	return arr
}

// --- operations ------------------------------------------------------------

// Merge merges several PDFs into one, in input order.
func Merge(pdfs [][]byte, opts *MergeOptions) ([]byte, error) {
	arr := cBytesArray(pdfs)
	if arr != nil {
		defer C.free(unsafe.Pointer(arr))
	}
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	r := C.dusrelpdf_merge(arr, C.size_t(len(pdfs)), o)
	runtime.KeepAlive(pdfs)
	return readResult(r)
}

// Split splits one PDF into one output per range. ranges is a list of
// page-number lists (1-based), e.g. [][]uint32{{1, 2}, {3, 4, 5}}.
func Split(pdf []byte, ranges [][]uint32, opts *PageSetOptions) ([][]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	rangesJSON, _ := json.Marshal(ranges)
	rj := C.CString(string(rangesJSON))
	defer C.free(unsafe.Pointer(rj))
	r := C.dusrelpdf_split(p, ln, rj, o)
	runtime.KeepAlive(pdf)
	return readResultList(r)
}

// SplitRanges splits one PDF from a range string such as "1-3,4-5" —
// one output per comma-separated chunk.
func SplitRanges(pdf []byte, ranges string, opts *PageSetOptions) ([][]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	rj := C.CString(ranges)
	defer C.free(unsafe.Pointer(rj))
	r := C.dusrelpdf_split_ranges(p, ln, rj, o)
	runtime.KeepAlive(pdf)
	return readResultList(r)
}

// SplitEachPage splits one PDF into one output per page.
func SplitEachPage(pdf []byte, opts *PageSetOptions) ([][]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	r := C.dusrelpdf_split_each_page(p, ln, o)
	runtime.KeepAlive(pdf)
	return readResultList(r)
}

// ExtractPages extracts selected pages into a new PDF, in the order given.
func ExtractPages(pdf []byte, pages []uint32, opts *PageSetOptions) ([]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	pj := cStringList(pages)
	defer C.free(unsafe.Pointer(pj))
	r := C.dusrelpdf_extract_pages(p, ln, pj, o)
	runtime.KeepAlive(pdf)
	return readResult(r)
}

// ExtractPagesRange extracts pages from a range string such as "1-3,5".
func ExtractPagesRange(pdf []byte, pageRange string, opts *PageSetOptions) ([]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	rj := C.CString(pageRange)
	defer C.free(unsafe.Pointer(rj))
	r := C.dusrelpdf_extract_pages_range(p, ln, rj, o)
	runtime.KeepAlive(pdf)
	return readResult(r)
}

// RemovePages deletes the given pages (1-based). An empty list returns
// the input unchanged.
func RemovePages(pdf []byte, pages []uint32, opts *PageSetOptions) ([]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	pj := cStringList(pages)
	defer C.free(unsafe.Pointer(pj))
	r := C.dusrelpdf_remove_pages(p, ln, pj, o)
	runtime.KeepAlive(pdf)
	return readResult(r)
}

// ReorderPages changes the page order. order must list every page
// exactly once, e.g. []uint32{3, 1, 2}.
func ReorderPages(pdf []byte, order []uint32, opts *PageSetOptions) ([]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	oj := cStringList(order)
	defer C.free(unsafe.Pointer(oj))
	r := C.dusrelpdf_reorder_pages(p, ln, oj, o)
	runtime.KeepAlive(pdf)
	return readResult(r)
}

// Rotate rotates pages by a multiple of 90 degrees. The angle (and the
// optional pages list) come from opts; nil opts rotate every page by 90.
func Rotate(pdf []byte, opts *RotateOptions) ([]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	r := C.dusrelpdf_rotate_pages(p, ln, o)
	runtime.KeepAlive(pdf)
	return readResult(r)
}

// GetPDFInfo reads a PDF's summary without modifying it.
func GetPDFInfo(pdf []byte, opts *CommonOptions) (*PdfInfo, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	r := C.dusrelpdf_get_pdf_info(p, ln, o)
	runtime.KeepAlive(pdf)
	data, err := readResult(r)
	if err != nil {
		return nil, err
	}
	var info PdfInfo
	if err := json.Unmarshal(data, &info); err != nil {
		return nil, &Error{Code: "OPERATION_FAILED", MessageEN: "could not parse PdfInfo: " + err.Error()}
	}
	return &info, nil
}

// ImagesToPDF builds a PDF with one page per image, in input order.
// Images are PNG/JPEG bytes.
func ImagesToPDF(images [][]byte, opts *ImagesToPdfOptions) ([]byte, error) {
	arr := cBytesArray(images)
	if arr != nil {
		defer C.free(unsafe.Pointer(arr))
	}
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	r := C.dusrelpdf_images_to_pdf(arr, C.size_t(len(images)), o)
	runtime.KeepAlive(images)
	return readResult(r)
}

// AddWatermark stamps a text or image watermark over the selected pages.
// Pass image bytes (PNG/JPEG) for an image watermark, or nil for a text
// watermark (set opts.Text). Exactly one of the two is required.
func AddWatermark(pdf []byte, image []byte, opts *WatermarkOptions) ([]byte, error) {
	p, ln := cBytes(pdf)
	ip, iln := cBytes(image)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	r := C.dusrelpdf_add_watermark(p, ln, ip, iln, o)
	runtime.KeepAlive(pdf)
	runtime.KeepAlive(image)
	return readResult(r)
}

// AddPageNumbers stamps page numbers over the selected pages.
// opts.Format must contain "{n}", e.g. "Page {n} of {total}".
func AddPageNumbers(pdf []byte, opts *PageNumberOptions) ([]byte, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	r := C.dusrelpdf_add_page_numbers(p, ln, o)
	runtime.KeepAlive(pdf)
	return readResult(r)
}

// PlaceSignature places a handwritten signature IMAGE on one page.
// This is image placement only — not a cryptographic digital signature.
// A transparent PNG keeps its transparency.
func PlaceSignature(pdf []byte, image []byte, opts *SignatureOptions) ([]byte, error) {
	p, ln := cBytes(pdf)
	ip, iln := cBytes(image)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	r := C.dusrelpdf_place_signature(p, ln, ip, iln, o)
	runtime.KeepAlive(pdf)
	runtime.KeepAlive(image)
	return readResult(r)
}

// BulkExtract extracts many selections from one PDF in a single call.
// One bad selection fails only its own item, never the whole batch.
// Each item's PDF is base64-decoded from the native JSON payload.
func BulkExtract(pdf []byte, selections [][]uint32, opts *PageSetOptions) ([]BulkItem, error) {
	p, ln := cBytes(pdf)
	o := cStringOf(opts)
	if o != nil {
		defer C.free(unsafe.Pointer(o))
	}
	selJSON, _ := json.Marshal(selections)
	sj := C.CString(string(selJSON))
	defer C.free(unsafe.Pointer(sj))
	r := C.dusrelpdf_bulk_extract(p, ln, sj, o)
	runtime.KeepAlive(pdf)
	data, err := readResult(r)
	if err != nil {
		return nil, err
	}
	// The native payload carries PDFs as base64 strings; encoding/json
	// decodes []byte fields from base64 automatically.
	var raw []struct {
		Index     int    `json:"index"`
		PDF       []byte `json:"pdf"`
		ErrorCode string `json:"error_code"`
		Error     string `json:"error"`
	}
	if err := json.Unmarshal(data, &raw); err != nil {
		return nil, &Error{Code: "OPERATION_FAILED", MessageEN: "could not parse bulk results: " + err.Error()}
	}
	out := make([]BulkItem, len(raw))
	for i, item := range raw {
		out[i] = BulkItem(item)
	}
	return out, nil
}
