# WASM extensions — design + contract

> **Audience:** Maintainer, contributor.
> **Status:** Stage 1 (build-flag variants).  Stage 2 (separately-merged
> extension WASMs) is sketched at the end and explicitly deferred.

This doc describes how the Tcl→WASM compiler ships *optional*
runtime features the user's program can request via
`package require`.  The first user is **Tcltest** — the full C-tier
`test*` command surface ported from `generic/tclTest.c` /
`tclTestObj.c` / `tclTestProcBodyObj.c` / `tclTestABSList.c` to
the Rust runtime's tcltest extension under `runtime/rust/`.  All 107
upstream commands are registered;
PORTABLE / PARTIAL ones have functional implementations, NOT-PORTABLE
ones (sockets, threads, fork, native FS hooks, hardware probes) raise
an explicit "not supported under WASM" error so test scripts get a
clear reason rather than `invalid command name`.

## Why optional extensions

Every command compiled into the runtime adds binary size and
attack surface.  The Tcl 9 `tcltest` C component alone is ~107
commands worth of test scaffolding (refcount probes, internal
parser introspection, abstract-list test types) that real user
programs never call.  Building it into every runtime instance —
the way the runtime currently bundles e.g. `string`, `list`, `dict`
— would penalise every user program with code that only the Tcl
test corpus exercises.

The link-time-optional model says: the user's program signals
which features it needs (via `package require`), the bundler
notices, and only then does the optional code end up in the
final `.wasm`.

## How it works today (Stage 1: variant runtimes)

Two artefacts emerge from the Rust runtime build (`make runtime-rust-test`,
i.e. `cargo build` in `runtime/rust/`):

| Artefact                              | command-table shape | Use case                              |
|---------------------------------------|------------------|---------------------------------------|
| `tcl_runtime.wasm`                    | lean             | Programs that don't `package require Tcltest` |
| `tcl_runtime_with_tcltest.wasm`       | lean + tcltest cmds | Programs that do                  |

The split is a Cargo feature (`tcltest`): `runtime/rust/src/builtins.rs`
uses `#[cfg(feature = "tcltest")]` to splice the tcltest command table in
or out at compile time.  The lean variant pays
zero space cost — the tcltest sources are never compiled.

Selection happens in :mod:`compiler.codegen.wasm.extensions`:

1. `find_required_extensions(ir_module)` walks the merged IR for
   `package require <name>` calls.
2. `runtime_path_for(ir_module)` maps that result to the right
   pre-built runtime artefact.
3. :func:`wasm_link_bundled` in `compiler/codegen/wasm/link.py`
   feeds the chosen runtime to Binaryen `wasm-merge`, fusing it
   with the user-code module into one `.wasm`.

## Adding a new extension (Stage 1)

1. Create a module under `runtime/rust/src/` (e.g. `cmd_<extname>.rs`)
   exporting a command-table registration in the same shape as the
   runtime's existing `cmd_*.rs` modules.  They use the runtime's
   internal Rust API directly (no `extern` plumbing — they'll be
   compiled into the runtime).

2. In `runtime/rust/src/builtins.rs`, add a
   `#[cfg(feature = "<extname>")]` import and append its registrations
   to the command table.

3. In `runtime/rust/Cargo.toml`:
   - Add a `<extname>` entry to `[features]` (off by default).
   - The feature-gated build (driven by `runtime/rust/build.rs`)
     produces the sibling `tcl_runtime_with_<extname>.wasm` artefact.
   - Wire the feature into the test config so unit tests can
     drive the new commands.

4. In `compiler/codegen/wasm/extensions.py`, append an
   `ExtensionDescriptor(name=…, package_names=…,
   runtime_path_factory=…)` entry to `EXTENSIONS`.

5. Add unit tests (e.g. `#[cfg(test)]` modules, or files under
   `runtime/rust/tests/`) gated on `#[cfg(feature = "<extname>")]`
   so they no-op on the lean build.

6. Add a `tests/test_wasm_bundle.py`-style end-to-end smoke
   test that compiles a Tcl program with the new
   `package require` and asserts the runtime variant kicks in.

7. Document the extension under `docs/design/compiler/`.

## Trade-offs of the variant-runtime model

* **Combinatorial blow-up.**  N extensions multiply the artefact
  count by 2ⁿ.  At 2 extensions we'd have 4 runtime variants;
  at 3 extensions, 8.  Tractable now (tcltest is the only one);
  not a long-term answer.

* **Build cost.**  Every `cargo build` recompiles each variant
  from scratch.  Two variants ≈ 12-15s on a warm cache; manageable.

* **Memory footprint.**  Each variant carries every cmd module's
  data — there's no de-duplication across artefacts.

The combinatorial issue is the chief reason Stage 2 (below) is
on the roadmap.

## Stage 2 — separately-merged extension WASMs (deferred)

The original plan was to compile each extension as a standalone
`.wasm` (e.g. `tcl_tcltest.wasm`) and have the bundler `wasm-merge`
it alongside the runtime + user code.  That approach hits two
issues that the build-flag variant sidesteps:

1. **Multi-memory.**  The `wasm32-wasip1` target makes each
   WASM module declare its own linear memory.  After
   `wasm-merge`, a separately-compiled extension keeps its memory
   too — which means a name string written into the extension's
   memory is at the same offset *in a different memory* than the
   runtime reads from when the extension calls
   `tcl_register_extension_command`.  Pointer round-trips break.

2. **Memory-import + data-placement.**  Solving (1) by setting
   `--import-memory` + `--global-base=N` requires N to be both
   above the runtime's heap working set *and* below the runtime's
   exported initial-memory size.  Running into "minimum memory
   size mismatch" on every realistic N — we'd need shared-memory
   negotiation at build time, which neither the `wasm32` toolchain nor
   `wasm-merge` v123 currently support.

A dynamic-linking-style design (function-table-only handover,
zero shared static data, names allocated through the runtime's
heap during init) would dodge (1) and (2), at the cost of a
substantially larger refactor of the per-extension Rust code.

Stage 2 lands when the matrix of variants becomes painful (likely
once a 2nd extension shows up and triples the artefact count).
The :class:`extensions.ExtensionDescriptor` already abstracts the
"which artefact to bundle" decision behind a factory — the only
contract change is that the factory will return a separate
extension `.wasm` instead of a runtime variant, and the bundler
will pass a list of extension paths to `bundle_wasm` instead of
just a runtime.

## Tcltest layout

The full upstream tcltest surface is split into 12 command groups
(in the Rust runtime's tcltest extension under `runtime/rust/`):

```
slots           — per-extension Tcl_Obj* slot table
cmd_obj         — testintobj / testbooleanobj / testdoubleobj /
                  testbignumobj / testindexobj / testlistobj /
                  testobj / teststringobj / testbigdata
cmd_eval        — testevalex / testevalobjv / testreturn /
                  testseterr / testsetnoerr / testset2 /
                  testseterrorcode / testsetobjerrorcode /
                  testwrongnumargs
cmd_expr        — testexprlong / testexprlongobj / testexprdouble /
                  testexprdoubleobj / testexprstring / testconcatobj
cmd_utf         — testutfnext / testutfprev / testnumutfchars /
                  testgetunichar / testfindfirst / testfindlast /
                  testuniclass
cmd_misc        — testlongsize / testsize / testgetint /
                  testgetintforindex / testgetindexfromobjstruct /
                  testdoubledigits / testlutil / testmsb /
                  testpurebytesobj / testbytestring /
                  teststringbytes / testsetbytearraylength /
                  testapplylambda / testpreferstable / testlocale /
                  testbumpinterpepoch / testdcall / testpanic /
                  testprint / testparseargs / testgetplatform /
                  testsetplatform / testhashsystemhash /
                  testhandlecount / testappverifierpresent /
                  testmainthread / testnrelevels / testnreunwind /
                  gettimes
cmd_dstring     — testdstring (full sub-command coverage)
cmd_assoc       — testsetassocdata / testgetassocdata /
                  testdelassocdata
cmd_var         — testupvar / testgetvarfullname
cmd_proc        — tcl::procbodytest::proc / tcl::procbodytest::check
cmd_abslist     — lstring / lgen / value:at:
cmd_parser      — testparser / testparsevar / testparsevarname /
                  testexprparser
cmd_cmdinfo     — testcmdinfo / testcmdtoken / testcmdtrace /
                  testcmdobj2 / testcreatecommand / testdel /
                  testinterpdelete / testinterpresolver
cmd_extra       — ::tcl::test::build-info /
                  test_ns_basic::createdcommand / testencoding /
                  testregexp / testlistrep
cmd_stubs       — NOT-PORTABLE stubs (testsocket / testcpuid /
                      testfevent / testevent / testsetmainloop /
                      testexitmainloop / testexithandler /
                      testservicemode / teststaticlibrary / testlink /
                      testlinkarray / testchannel / testchannelevent /
                      testfilesystem / testsimplefilesystem / testfile /
                      testfilelink / testfstildeexpand /
                      testtranslatefilename / testasync) plus the
                      ``noop`` trivial command.
```

## Verification

1. `make runtime-rust-test` (i.e. `cargo build` in `runtime/rust/`)
   produces both `tcl_runtime.wasm` and `tcl_runtime_with_tcltest.wasm`.
2. `make runtime-rust-test` (`cargo test` in `runtime/rust/`) runs the
   tcltest unit tests under `wasmtime`.
3. `make check-wasm-parity` — extension-aware parity check
   accepts the variant runtime's expanded BUILTINS.
4. Bundle smoke coverage — three cases: (a) lean bundle without
   extensions, (b) bundle with `package require Tcltest` running real
   test commands, (c) lean bundle correctly rejecting test commands when
   `package require` is absent.

> **Update (2026):** Python has been fully retired on this branch. The
> old `uv run pytest tests/test_wasm_bundle.py` smoke driver is gone;
> the bundle coverage above now runs natively via the Rust runtime tests
> (`cargo test` / `make runtime-rust-test`).
