# Security Policy

## Reporting a vulnerability

Open a GitHub issue on `dusmamud/dusrelpdf` describing the problem
and a minimal reproducing PDF, or email the maintainer via the
address on the GitHub profile. Do not open a public issue with an
exploit that affects released versions before it is fixed.

## Scope notes

- dusrelpdf parses **untrusted PDF files** with lopdf (pure Rust).
  The core is written to never panic on malformed input — a bad PDF
  returns `INVALID_PDF`, it does not crash the process. If you find
  an input that panics or hangs, that is a security bug: please
  report it.
- Whole documents are loaded into memory; a decompression bomb is
  bounded by lopdf's per-stream limit (`DEFAULT_MAX_DECOMPRESSED_SIZE`).
- Encrypted PDFs are only opened with a caller-supplied password.
  dusrelpdf never tries to crack, bypass or remove encryption.
- The signature feature places a raster image; it provides no
  authenticity or integrity guarantees.
