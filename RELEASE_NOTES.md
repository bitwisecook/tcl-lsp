# v1.10.7

## Bug Fixes

- **Compiled `switch` over-release.** The bytecode emitter for `switch`
  could release the matched `TclObj` one extra time, corrupting the
  free list under workloads that hit the default branch repeatedly.
  A regression test pinned the failure and the codegen now balances
  the refcount across every arm.
- **`return -code break` / `-code continue` from compiled procs.**
  Compiled proc calls swallowed the propagated `break`/`continue`
  return codes from `return -code`, so loops driven by helpers that
  used this pattern terminated incorrectly. The interpreter result
  now flows through unchanged.
- **`TclObj` refcount leaks across runtime and codegen.** A sweep of
  the runtime and codegen paths fixed several places where intermediate
  `TclObj` values weren't released — including `eval`/`set` ownership
  and a handful of arithmetic helpers — so long-running scripts no
  longer drift upward in resident objects.
- **`expr` error messages and boolean-context handling.** Error text
  for malformed expressions and the boolean coercion in test/loop
  contexts now match C Tcl, including the `expected boolean value`
  wording and operand position.
- **Alias loops and cross-interp `upvar`.** The interpreter now
  detects alias cycles introduced by `interp alias` and resolves
  `upvar` correctly when the target frame lives in a sibling
  interpreter.
- **`list` validation and `lsearch -bisect`.** Argument-count
  validation for `list` and the bisection mode of `lsearch` now agree
  with Tcl 9 semantics, including boundary behaviour on empty and
  duplicated keys.
- **E003 false positives when switches precede positional args.** The
  arity diagnostic no longer trips when option switches (e.g. `-nocase`)
  come before positional arguments to commands like `lsearch` and
  `string match`.
- **`rename` preserves array indices.** Renaming the base variable of
  an array reference (`set a(b) …` → `set c(b) …`) now keeps the
  index intact instead of dropping it.
- **JetBrains: Compiler Explorer receives source from the editor.**
  Since v1.10.0 the tool window stayed stuck on "Waiting for source
  from editor..." with the status line reading "webview API
  unavailable". The JCEF adapter in `CompilerExplorerHtml.kt` was
  rewriting a `const vscode = acquireVsCodeApi();` string that the
  current VS Code webview HTML no longer contains. The adapter now
  injects a `<head>` shim that defines `acquireVsCodeApi` to return a
  bridge object whose `postMessage` queues requests until the Kotlin
  host installs `window.__tcllspBridge`, at which point the queue is
  drained.
- **JetBrains: bundle LSP server outside the plugin jar.** v1.10.6
  shipped the source-level W105 quick-fix that preserves `$` in
  variable references (`$script` → `{$script}`), but users running the
  JetBrains plugin didn't see it on upgrade: the plugin extracted
  `tcl-lsp-server.pyz` to `${tmpdir}` at first launch and only
  re-extracted when the temp file was missing. The pyz now lives at
  the plugin install root (next to `lib/`), so Python executes it
  directly — no temp-dir cache, no upgrade-time invalidation.

## Improvements

- **`mathop` operators delegate to `tcl_arith` helpers.** The
  `::tcl::mathop` namespace no longer duplicates arithmetic; every
  operator now routes through the shared `tcl_arith` paths, bringing
  operator semantics into line with `expr` and shrinking the divergence
  surface between the two.

## Internal

- **Double-free sub-bucket counters in the leak-check runtime.**
  `leak_sweep` and the diff tooling now classify double-free events
  into sub-buckets and capture pending sample data, making
  regressions easier to localise.
- **Apply `zig fmt`** to `runtime/zig/cmds/dict.zig` and
  `runtime/zig/cmds/tcl_mathop.zig`.
- **Ignore JetBrains `tcl-lsp-server.pyz`** build artefact in the
  source tree.

# v1.10.6

## Bug Fixes

- **Quick-fix range widening for unbraced-expression diagnostics.** The
  W100 (`expr` argument needs braces) and W101 (control-flow body needs
  braces) auto-fixes previously stopped one character short of the
  closing `"` / `}`, leaving a stray delimiter in the document after
  the rewrite. They also rewrote arguments using the *post-substitution*
  value, silently dropping `$var` / `[cmd]` references. Both now widen
  the replacement range correctly and preserve the original
  substitution syntax verbatim.
- **`matchclass` → `class match` quick-fix preserves substitutions.**
  IRULE2001's auto-rewrite was producing `class match url equals ::lib`
  from source `matchclass $url ::lib` — silently turning a variable
  reference into a literal. The rewrite now reads the raw token text
  so `$url` round-trips intact, and the fix range covers the closing
  delimiter.

## Improvements

- **JetBrains plugin: auto-restart on settings change** (#438). The
  LSP server now restarts automatically when the resolved Python path
  changes in settings, and the resolved interpreter is logged at
  startup so discovery issues are easier to diagnose.

## Internal

- **Release process now gates on CodeQL** (#440). A new
  `release-codeql-gate` Makefile target watches the CodeQL run for
  the tag candidate on `main` and blocks the release if any open
  alert is high or critical severity. Overridable via
  `CODEQL_GATE_MIN_SEVERITY=critical` for documented exceptions.
