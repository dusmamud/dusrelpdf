// napi-build emits the platform linker flags the cdylib needs — on
// macOS that is `-undefined dynamic_lookup`, without which the link
// fails with unresolved `_napi_*` symbols (Node provides them at
// load time; Linux/Windows linkers tolerate the undefined symbols,
// macOS `ld` does not). See napi-rs prebuilds run 43383594472
// (darwin-arm64 + darwin-x64 failed at `cargo build` for this reason).
fn main() {
    napi_build::setup();
}
