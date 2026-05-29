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
- **JetBrains: Compiler Explorer source push no longer races the EDT.**
  Even with the JCEF bridge in place, the explorer could stay on
  "Waiting for source from editor..." when it was opened while a Tcl
  file was already the active tab: the initial push ran from the JCEF
  `onLoadEnd` callback thread and read `selectedTextEditor` off the
  EDT, so it silently produced nothing and no editor event ever fired
  to retry. The push now hops to the EDT, and the panel also listens
  for `selectionChanged` so switching between already-open tabs
  recompiles.
- **JetBrains: bundle LSP server outside the plugin jar.** v1.10.6
  shipped the source-level W105 quick-fix that preserves `$` in
  variable references (`$script` → `{$script}`), but users running the
  JetBrains plugin didn't see it on upgrade: the plugin extracted
  `tcl-lsp-server.pyz` to `${tmpdir}` at first launch and only
  re-extracted when the temp file was missing. The pyz now lives at
  the plugin install root (next to `lib/`), so Python executes it
  directly — no temp-dir cache, no upgrade-time invalidation.
- **Refactor edits no longer corrupt source.** Applying refactors
  through the real editor surfaced three transformation bugs: *extract
  variable* on a bare operator expression emitted an invalid
  `set result $a * $b` (now wrapped in `[expr { ... }]`); *inline
  variable* into a quoted string produced broken nested quotes like
  `"hello "world""` (now splices the unquoted content, and refuses an
  unsafe space-splitting concatenation); and *inline proc* dropped
  structural body braces (`expr {$x * 2}` collapsed to `expr $x * 2`,
  now substitutes into the raw body span). Inline variable was also
  rewritten to decide the reference context from tokens rather than
  regex, so `${name}` braced references with trailing text and uses
  inside nested bodies now rewrite correctly.
- **`extract proc` overlapping-edit fix.** Extracting from the top of
  the file emitted a zero-width insert and a replace sharing the same
  offset (a client-order-dependent overlapping edit); the two are now
  merged into a single edit.
- **W110 quick-fix brace mangling.** The string-comparison fix
  (`==`/`!=` → `eq`/`ne`) rewrote `{$x == "foo"` to brace-free text,
  producing the broken `if $x eq "foo"} ...`. The fix range is now
  content-aligned for braced `if`/`while` conditions, preserving the
  braces.
- **`${name}` references resolve for navigation and rename.** Go-to-
  references and rename did not work from a braced `${name}` reference
  (the backward word-scan stopped at the brace); the braced form,
  including `${arr(k)}`, now resolves correctly.
- **Rename rewrites bareword write sites.** Writes to an
  already-defined variable via `incr`/`append`/`lappend` or a repeated
  `set` were never recorded as references, so find-references and
  rename missed them. Rename now rewrites every occurrence —
  definition, `$`-reads, `${}`-reads, and bareword writes — across all
  variable forms.
- **Namespace-qualified variable reads resolve.** A read of `$ns::v` /
  `$::ns::v` / `$a::b::v` now resolves to the variable defined in the
  target `namespace eval` block for references and rename, across
  relative, absolute, and nested namespace paths.
- **Nested-namespace proc qualification.** A proc inside
  `namespace eval a { namespace eval b { ... } }` was qualified using
  only the innermost namespace (`::b::proc`), breaking call-hierarchy
  and reference links for `a::b::proc`; qualified names now use the
  full namespace path. Call hierarchy also no longer cross-links
  same-named procs in sibling namespaces.
- **Class references in `superclass`/`mixin` declarations.** Find-
  references and rename for a class now span its uses in other classes'
  `superclass`/`mixin` declarations (both `oo::class create` bodies and
  `oo::define` forms), not just its definition and constructor calls.

## Improvements

- **"Open In Tcl Compiler Explorer" context-menu entry.** Both editor
  plugins now expose the Compiler Explorer from a right-click. JetBrains
  adds an "Open In Tcl Compiler Explorer" action to the editor and
  project-view popups that reveals the tool window and pushes the file;
  VS Code's existing entry is renamed to "Open in Tcl Compiler Explorer"
  to match.
- **`mathop` operators delegate to `tcl_arith` helpers.** The
  `::tcl::mathop` namespace no longer duplicates arithmetic; every
  operator now routes through the shared `tcl_arith` paths, bringing
  operator semantics into line with `expr` and shrinking the divergence
  surface between the two.
- **Sharper diagnostic ranges.** A sweep narrowed many diagnostics from
  the whole statement to the exact offending token, so squiggles point
  at the real problem: the variable-lifecycle codes (W210 read-before-
  set, W211 set-but-unused, W213 unset-may-not-exist, W220 dead store),
  W001 (unknown subcommand), W302 (catch without result), the taint
  sinks (T100/T101, W101/W102, W309/W312, IRULE3001/3002), and others.
  Braced-context codes (W110 string comparison, W240/W241/W242 loop
  conditions, W309 `subst`) now include the closing brace/bracket the
  lexer omits rather than dropping it.

## Internal

- **Tokeniser-based parsing replaces ad-hoc regexes.** A robustness
  sweep replaced hand-rolled regular expressions with the shared lexer
  across the refactoring and analysis paths — `switch`→`dict` and
  `extract-datagroup` arm bodies, tail-call (O121) rewriting, SCCP
  command-substitution folding, redundant-`expr` (W114) detection,
  minifier array-member splitting, and the simple-variable-name checks
  in `proc_arg_traits`, `static_loops`, and `interprocedural`. Behaviour
  is unchanged on valid input and more correct on malformed input (e.g.
  the old regexes wrongly accepted single-colon names). Dead regexes
  were removed.
- **Diagnostic range-coverage enforcement framework.** All registered
  diagnostic codes are now partitioned into verified-range fixtures,
  a shrinking backlog, and not-yet-covered buckets, with a partition
  test that fails if any code is unclassified — so new codes cannot
  ship without range scrutiny.
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
