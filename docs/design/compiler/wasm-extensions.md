# WASM extensions — design + contract

> **Audience:** Maintainer, contributor.
> **Status:** Stage 1 (build-flag variants).  Stage 2 (separately-merged
> extension WASMs) is sketched at the end and explicitly deferred.

This doc describes how the Tcl→WASM compiler ships *optional*
runtime features the user's program can request via
`package require`.  The first user is **Tcltest** — the full C-tier
`test*` command surface ported from `generic/tclTest.c` /
`tclTestObj.c` / `tclTestProcBodyObj.c` / `tclTestABSList.c` to
`runtime/zig/tcltest/`.  All 107 upstream commands are registered;
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

Two artefacts emerge from `cd runtime/zig && zig build`:

| Artefact                              | `BUILTINS` shape | Use case                              |
|---------------------------------------|------------------|---------------------------------------|
| `tcl_runtime.wasm`                    | lean             | Programs that don't `package require Tcltest` |
| `tcl_runtime_with_tcltest.wasm`       | lean + tcltest cmds | Programs that do                  |

The split lives in `runtime/zig/build.zig`: each artefact has its
own `build_options` set, and `dispatch/tcl_cmd_table.zig` ``inline
if``s on `build_options.with_tcltest` to splice the tcltest
`BUILTINS` slice in or out at comptime.  The lean variant pays
zero space cost — the tcltest sources are never touched.

Selection happens in :mod:`core.compiler.codegen.wasm.extensions`:

1. `find_required_extensions(ir_module)` walks the merged IR for
   `package require <name>` calls.
2. `runtime_path_for(ir_module)` maps that result to the right
   pre-built runtime artefact.
3. :func:`wasm_link_bundled` in `core/compiler/codegen/wasm_link.py`
   feeds the chosen runtime to Binaryen `wasm-merge`, fusing it
   with the user-code module into one `.wasm`.

## Adding a new extension (Stage 1)

1. Create `runtime/zig/<extname>/` with one or more `cmd_*.zig`
   files that export `pub const registrations: [_]reg.CmdEntry`
   in the same shape as the runtime's existing
   `runtime/zig/cmds/*.zig` modules.  They use the runtime's
   internal Zig API directly (no `extern` plumbing — they'll be
   compiled into the runtime).

2. In `runtime/zig/dispatch/tcl_cmd_table.zig`, add an
   ``inline if`` import gated on `build_options.with_<extname>`,
   and append its `registrations` to the `BUILTINS` slice.

3. In `runtime/zig/build.zig`:
   - Add a `with_<extname>` option to the lean runtime's
     `build_options` (default `false`).
   - Clone the runtime's `addExecutable` block into a sibling
     target named `tcl_runtime_with_<extname>`, with its own
     `build_options` setting `with_<extname> = true`.
   - Add the same option to `test_options` so unit tests can
     drive the new commands.

4. In `core/compiler/codegen/wasm/extensions.py`, append an
   `ExtensionDescriptor(name=…, package_names=…,
   runtime_path_factory=…)` entry to `EXTENSIONS`.

5. Add unit tests at `runtime/zig/test_<extname>_*.zig` (auto-
   discovered by `build.zig`'s test walker — gate them on
   `build_options.with_<extname>` so they no-op on the lean
   build).

6. Add a `tests/test_wasm_bundle.py`-style end-to-end smoke
   test that compiles a Tcl program with the new
   `package require` and asserts the runtime variant kicks in.

7. Document the extension under `docs/design/compiler/`.

## Trade-offs of the variant-runtime model

* **Combinatorial blow-up.**  N extensions multiply the artefact
  count by 2ⁿ.  At 2 extensions we'd have 4 runtime variants;
  at 3 extensions, 8.  Tractable now (tcltest is the only one);
  not a long-term answer.

* **Build cost.**  Every `zig build` recompiles each variant
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

1. **Multi-memory.**  Zig 0.16's wasm32-wasi target makes each
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
   negotiation at build time, which neither Zig 0.16 nor
   `wasm-merge` v123 currently support.

A dynamic-linking-style design (function-table-only handover,
zero shared static data, names allocated through the runtime's
heap during init) would dodge (1) and (2), at the cost of a
substantially larger refactor of the per-extension Zig code.

Stage 2 lands when the matrix of variants becomes painful (likely
once a 2nd extension shows up and triples the artefact count).
The :class:`extensions.ExtensionDescriptor` already abstracts the
"which artefact to bundle" decision behind a factory — the only
contract change is that the factory will return a separate
extension `.wasm` instead of a runtime variant, and the bundler
will pass a list of extension paths to `bundle_wasm` instead of
just a runtime.

## Tcltest layout

The full upstream tcltest surface is split across 12
`runtime/zig/tcltest/` files:

```
slots.zig           — per-extension Tcl_Obj* slot table
cmd_obj.zig         — testintobj / testbooleanobj / testdoubleobj /
                      testbignumobj / testindexobj / testlistobj /
                      testobj / teststringobj / testbigdata
cmd_eval.zig        — testevalex / testevalobjv / testreturn /
                      testseterr / testsetnoerr / testset2 /
                      testseterrorcode / testsetobjerrorcode /
                      testwrongnumargs
cmd_expr.zig        — testexprlong / testexprlongobj / testexprdouble /
                      testexprdoubleobj / testexprstring / testconcatobj
cmd_utf.zig         — testutfnext / testutfprev / testnumutfchars /
                      testgetunichar / testfindfirst / testfindlast /
                      testuniclass
cmd_misc.zig        — testlongsize / testsize / testgetint /
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
cmd_dstring.zig     — testdstring (full sub-command coverage)
cmd_assoc.zig       — testsetassocdata / testgetassocdata /
                      testdelassocdata
cmd_var.zig         — testupvar / testgetvarfullname
cmd_proc.zig        — tcl::procbodytest::proc / tcl::procbodytest::check
cmd_abslist.zig     — lstring / lgen / value:at:
cmd_parser.zig      — testparser / testparsevar / testparsevarname /
                      testexprparser
cmd_cmdinfo.zig     — testcmdinfo / testcmdtoken / testcmdtrace /
                      testcmdobj2 / testcreatecommand / testdel /
                      testinterpdelete / testinterpresolver
cmd_extra.zig       — ::tcl::test::build-info /
                      test_ns_basic::createdcommand / testencoding /
                      testregexp / testlistrep
cmd_stubs.zig       — NOT-PORTABLE stubs (testsocket / testcpuid /
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

1. `cd runtime/zig && zig build` produces both `tcl_runtime.wasm`
   and `tcl_runtime_with_tcltest.wasm` under `zig-out/bin/`.
2. `cd runtime/zig && zig build test` runs `test_tcltest_*.zig`
   under `wasmtime`.
3. `make check-wasm-parity` — extension-aware parity check
   accepts the variant runtime's expanded BUILTINS.
4. `uv run pytest tests/test_wasm_bundle.py` — three smoke tests
   covering: (a) lean bundle without extensions, (b) bundle
   with `package require Tcltest` running real test commands,
   (c) lean bundle correctly rejecting test commands when
   `package require` is absent.
