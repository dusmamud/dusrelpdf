# Contributing to dusrelpdf

## Ground rules

- **One core, one truth.** All PDF logic lives in
  `crates/dusrelpdf`. JS, Python and the CLI are thin wrappers — never
  re-implement an operation in a binding. If the Rust core and a
  binding disagree, the binding is wrong.
- **Never panic on bad input.** Every public function returns a
  `DusrelError` with a stable code and EN + HI reasons. Test the
  error paths, not just the happy paths.
- **Bytes in, bytes out.** Operations take `&[u8]` and return
  `Vec<u8>` (or `Vec<Vec<u8>>` for split). File-path helpers live in
  the CLI and the bindings, not in the core.
- **Tests prove it.** A new operation needs Rust tests on fixture
  PDFs (built with `dusrelpdf::fixtures`), plus vitest and pytest
  coverage. Re-load every output and verify its pages — "it returned
  bytes" is not a test.
- **Honest limits.** If something does not work (outlines on merge,
  encrypted PDFs, ASCII-only stamps), document it in the README,
  the website limitations page and the CHANGELOG — do not hide it.

## Workflow

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Version bumps are done by release-plz (`release-pr` flow) — do not
bump versions by hand. Never publish to npm / PyPI / crates.io from a
local checkout; releases go through the GitHub workflows.
