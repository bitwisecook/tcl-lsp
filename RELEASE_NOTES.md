# v1.8.0

## New Features
- Per-folder configuration: `tclLsp.*` settings now resolve per workspace
  folder, so multi-folder VS Code workspaces honour distinct formatter and
  diagnostic configuration per folder. Pull-config now happens on
  initialisation via the pygls 2.x `Workspace.folders` API.
- Cross-file namespace imports: the LSP now resolves `namespace import`
  across files, including tcllib-style factory procs.
- Variable traces on proc-locals: runtime supports `trace add variable`
  on local variables in compiled procs.
- `info frame` in compiled procs: typed result snapshot and per-frame
  metadata enable accurate `info frame` and `info level` reporting,
  including `info level 0` argv.
- `interp share-variable` and coroutine frame isolation.
- Full `lsearch` options support, and multi-iterator `foreach`
  (`foreach a $l1 b $l2 {...}`).
- Arbitrary-precision integers in `expr`: bignum support spans
  comparison, bitwise, shift, `pow`, `int()` coercion, `mathop`, `incr`,
  `format`, `scan`, and `string is integer/wideinteger`.
- Combined `-nocase -length` form on `string compare` / `string equal`.
- Child interpreters: full delete cascade, child-as-command dispatch,
  per-parent `idIssuer`, and `interp.test` parity.
- Eglot debugging support: bug-recorder and headless test harness for
  upstream eglot issue #333, plus README workaround documentation.

## Improvements
- Diagnostic suppression: `noqa` / next-line annotations now cover
  orphaned comments and the noqa-before-comment case (issue #306), with
  scope for all suppression layers documented in the README.
- WASM coverage: many more runtime commands implemented; substantial
  fixes to `cmdAH`, `opt.test`, `dict` subcommands, `file` subcommand
  abbreviations, and multi-token quoting.
- Error handling: automatic `errorCode TCL WRONGARGS` tagging, dynamic
  switch handler, and proper routing of user `return -code error` through
  the 3-arg error sink.
- Tcl 9 test suite: two null-pointer traps fixed and eight previously
  failing suites now gated and passing.
- Internal architecture: command registry-driven dispatch in both the
  WASM emitter and the Zig runtime; large modules
  (`core/analysis/analyser.py`, `lsp/server.py`, `wasm/_emitter.py`,
  `runtime/zig/tcl_interp.zig`) split into focused leaf modules and
  mixin packages, improving maintainability.
- Compiled-frame `upvar` / `uplevel` correctness for `opt.test`, with
  signal propagation across compiled-proc boundaries.
- Build: dropped `python-minifier` from the zipapp pipeline.
- Security: pinned `@azure/msal-node` to `^5.1.5` to address the
  `uuid<14` advisory in the VS Code extension.

## Bug Fixes
- Memory: plugged `TclObj` / buffer leaks across proc-call, list, error,
  and parse-cache paths (issue #317); `frame_depth_restore` now releases
  the orphan frame buffer; `obj_new_string_take` frees its buffer on
  OOM; `parse_cache` gates `free_bucket_slab` on occupied buckets.
- `upvar` rejects negative levels with a bad-level error; non-literal
  upvar levels are treated as dynamic in `var_escape`.
- `arr(key)` reads and writes are routed through the array directory in
  `var_*`.
- `double(x)` raises on non-numeric coercion.
- Codegen folds `\<newline>` line continuations inside interpolated
  strings; multi-element list literals retain their braces; `catch`
  return-code propagation fixed in WASM.
- Six pre-existing Zig test-suite failures resolved.
- `var_escape`: literal-target `uplevel` writes (e.g. `uplevel 1 set x …`)
  are now detected by the CFG analysis.
