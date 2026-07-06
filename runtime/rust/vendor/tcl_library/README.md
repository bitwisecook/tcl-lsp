# Vendored Tcl 9 standard library (subset)

These are **unmodified** files from the C Tcl 9.0.3 `library/` tree, vendored so
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

## Licence

Tcl is distributed under the BSD-style Tcl licence — see
[`license.terms`](license.terms). These files are copyright their original
authors (the Tcl Core Team and contributors) and are redistributed unmodified
under that licence.

## Updating

To re-vendor for a new Tcl version, copy the same file set from
`tmp/tcl<version>/library/` (fetched by
`.claude/skills/fetch-tcl-source/fetch_tcl_source.sh`). If a new bootstrap path
reads additional files, add them here and to the `FILES` table in
`embedded_stdlib.rs`.
