# v1.10.8

## New Features

- **Tcl 9 source forms & child-interp init.** WASM `source` now accepts
  `-encoding enc` and `-nopkg`, and trusted child interpreters run a
  best-effort `Tcl_Init` that sources `init.tcl` (mirroring C's `ChildCreate`)
  when the parent has loaded `init.tcl` itself.
- **Sublime Text key bindings & context-menu filtering.** Ships an
  `Example (*).sublime-keymap` with commented suggested bindings, a
  Key Bindings entry under the Preferences menu, and scope-gated context
  menu entries (`source.tcl` / `source.irule`) via per-command
  `is_visible()` rather than the inert `context` key. New KCS how-to
  documents the full list of bindable commands.
- **JetBrains "Open In Tcl Compiler Explorer" action.** Right-click any
  Tcl file in the editor or project view to push it into the live
  Compiler Explorer panel; VS Code's existing command is renamed to
  match.
- **External `.tcl.stubs` files and `-loop` stub semantics.** Stubs
  declared with `-loop` and `body:body` are now lowered as foreach-shaped
  loops so the body is analysed in iteration order with the loop variable
  bound, instead of as an opaque barrier. Stub-declared commands that are
  disabled in the active dialect no longer get W002.

## Improvements

- **~10% faster analysis via shared tokenisation memo.** The analysis
  pipeline previously re-tokenised the same source bytes 3-4× across the
  segmenter, lowerer, and compiler-checks. A per-analysis token cache
  now serves all three from a single lex.
- **Dataflow fixpoints converted to worklists with shared RPO.** The
  duplicated/inconsistent block-ordering helpers across SCCP, GVN,
  liveness, taint, and interprocedural param-deps are replaced with one
  shared iterative reverse-postorder, and dominators are computed via
  Cooper-Harvey-Kennedy in O(N) memory. Large generated procs (85k+
  lines) that previously hung, OOM'd, or overflowed the stack now
  analyse under a 2 MB worker-thread stack.
- **JetBrains Compiler Explorer hardening.**
  - Polls for the lazy-started LSP server (kicked via
    `startServersIfNeeded`) before compiling, instead of surfacing a
    spurious "LSP server not running" on IDE startup.
  - Uses a monotonic wait deadline, bails if the project is disposed
    mid-wait, restores the interrupt flag on `InterruptedException`,
    starts the server before the clock so a busy EDT can't burn the
    budget, and runs the compile on IntelliJ's pooled-thread executor
    instead of the ForkJoinPool common pool.
  - Distinguishes "still starting" from "not running" in the timeout
    error.
- **Dependencies bumped to current versions.** Python: pygls, lsprotocol,
  argcomplete, pytest, pytest-cov, ruff, ty, import-linter, wasmtime,
  flask, cryptography, sphinx, myst-parser, sphinx-autodoc-typehints,
  furo. Zed: Cargo.lock refreshed. GitHub Actions: `github/codeql-action`
  4.35.5 → 4.36.0.
- **npm dependencies pinned via committed lockfile.** CI and
  `ensure-test-deps` use hash-pinned `npm ci` against
  `editors/vscode/package-lock.json`, resolving OpenSSF Scorecard
  `PinnedDependencies (npmCommand)`.

## Bug Fixes

- **W214 false positive on `switch` subject variables (#471).** A
  parameter read only as a `switch -- $col` subject was wrongly reported
  as unused — both for exact switches lowered to CFG branch conditions
  (`ExprRaw` text not scanned for var refs) and for `-glob` / `-regexp` /
  fallthrough switches kept as opaque `IRSwitch` (subject and arm bodies
  never entered the CFG).
- **E002/E003 false positives when a user proc shadows a builtin.**
  Defining e.g. `proc ::ns::close` previously silenced the builtin arity
  check on every same-named call, including global calls that actually
  resolve to the builtin. Arity suppression is now namespace-aware and
  gated on reachable, textually-preceding top-level proc definitions
  — fixing spurious E003 on tcllib's `websocket.tcl`.
- **`uplevel -0` in the WASM interpreter.** The signed-zero level
  specifier was previously routed to the body and ran "-0" as a command;
  now matches reference Tcl by resolving to the current frame.
- **Variable traces through `upvar` aliases.** Reads, writes, and unsets
  via an `upvar` alias now fire the target frame's proc-local traces;
  the callback reports the alias name used at the access site.
- **Stdlib procs bundled at compile time.** Removes a class of
  uplevel/upvar edge-case failures when stdlib procs were sourced
  lazily.
- **Compiler-explorer highlight ranges dropped trailing characters.**
  Braced conditions showed as `{$conditi` instead of `{$condition}`:
  semantic-model `Range` ends are inclusive but the explorer front-end
  slices with an exclusive end, and braced/quoted word ranges follow
  the codebase "inner-end" convention. Fixed in the explorer consumer
  (range serialiser widens via `widen_for_highlight` and converts
  inclusive→exclusive) so the optimiser, SCCP, structure-elimination,
  code-sinking, and minifier are unaffected.
- **Refactor edits dropping or mangling characters.** Apply-the-edit
  tests at the LSP, unit, and VS Code layers surfaced three semantic
  bugs that title/existence tests missed:
  - Extract variable on a bare operator expression emitted invalid
    `set result $a * $b`; now wraps operator expressions in `[expr { ... }]`.
  - Inline variable into a quoted string produced broken nested quotes
    (`"hello "world""`); now splices the unquoted value content for
    string-interpolation and bare-concatenation contexts.
  - Inline proc dropped structural body braces (`expr {$x * 2}` →
    `expr $x * 2`); now substitutes parameters into the raw body span.
- **W302 (`catch` without result) range narrowed** to the `catch`
  keyword itself rather than spanning the whole statement and dropping
  the closing brace.
- **`switch -regexp` lowering.** Kept as `IRSwitch` instead of
  collapsing to an `IRBarrier`, so SSA can recover subject and arm-body
  variable reads. The WASM backend re-invokes `switch` through the
  runtime eval fallback (preserving braced arm bodies via
  `_regexp_switch_eval_script`).
- **JetBrains Compiler Explorer "Waiting for source from editor…"** The
  initial source push ran on the JCEF onLoadEnd thread and read
  `selectedTextEditor` off the EDT. Now hops to the EDT, defers delivery
  until the JCEF page is ready, and listens for `selectionChanged` so
  switching between already-open tabs recompiles.
- **CI: `build-zipapp-wasm` tag-only job missing Zig.** v1.10.7's tag
  failed with `zig: command not found`, skipping `publish-checksums`
  and leaving that release without `SHA256SUMS`. The job now installs
  `setup-zig` and fetches the regex vendor sources, matching `pr-gate`.
- **CodeQL error-level findings.** Fixed a docstring `coding: pcap`
  cookie matching PEP 263 (`py/syntax-error`), a wrong-arity call in
  `profile_semantic_tokens.py`, mutating calls inside `assert` (stripped
  under `python -O`), and an unused loop variable. The compiler-explorer
  `esc()` helper now escapes single quotes for the `data-var='…'`
  attribute context (`js/incomplete-html-attribute-sanitization`).

## Breaking Changes

- **Python package layout reorganised into a seven-concern architecture
  (#449).** Internal imports moved (e.g. `vm/` → `tooling/vm/`,
  `tclpkg/` → `tooling/tclpkg/`, `explorer/wasm_cli.py` →
  `tooling/wasm/main.py`). External consumers that import from these
  modules directly will need to update import paths; users of the LSP
  server, CLI, and editor extensions are unaffected.
