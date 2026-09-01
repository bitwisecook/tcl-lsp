# Vendored Tcl 9 standard library (subset)

These are **unmodified** files from the C Tcl 9.0.4 `library/` tree, vendored so
the Rust runtime can embed them in the WASM binary (`--features wasm_stdlib`) and
seed them into the in-memory VFS — letting a self-contained `wasm32-wasip1`
module bootstrap the real standard library with no host filesystem. See
[`../../src/embedded_stdlib.rs`](../../src/embedded_stdlib.rs).

The set is exactly the **read-closure** of bootstrapping `init.tcl` and loading
the `tcltest` package (determined by tracing the file reads of a native
`run_script --init set.test`):

- `init.tcl` — the startup script (`unknown`/auto-load/`package`).
- `tclIndex` — the auto-load index for the library directory.
- `package.tcl`, `tm.tcl` — the `package`/Tcl-module machinery (`tclPkgUnknown`,
  `::tcl::tm::UnknownHandler`), auto-loaded via `tclIndex`.
- `parray.tcl` — auto-loaded by the bootstrap path.
- `*/pkgIndex.tcl` — the per-package indices the `package unknown` scan sources.
- `tcltest/tcltest.tcl` — the `tcltest` package itself.

The large data trees (`tzdata`, `encoding`, `msgs`) are **not** read by this path
and are omitted to keep the binary small.

[`manifest.json`](manifest.json) records the upstream tag and commit, each
source-relative path, its SHA-256 digest, and whether it is part of the embedded
read-closure. `cargo xtask runtime-stdlib` verifies the manifest, the exact
`init.tcl` patch requirement, and the `embedded_stdlib.rs` `FILES` table
offline; it is part of `make rust-check`.

## Licence

Tcl is distributed under the BSD-style Tcl licence — see
[`license.terms`](license.terms). These files are copyright their original
authors (the Tcl Core Team and contributors) and are redistributed unmodified
under that licence.

## Updating

To re-vendor for a new Tcl version, trace the read-closure from the official
source tree, copy those files without modification, and update `manifest.json`.
If the closure changes, update the `FILES` table in `embedded_stdlib.rs` too.
