# v2.1.6

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

This release is dominated by a deep, family-by-family review of the diagnostic
engine — every `E`, `W`, `S`, `T`, and `O` code was audited for false
positives, false negatives, highlight precision, and quick-fix coverage — plus
a rebuilt, parallelised language-server pipeline. Several defaults change as a
result; see **Breaking Changes**.

## New Features

- **Progressive, parallelised language-server pipeline.** A flicker-safe fast
  tier of diagnostics publishes ahead of the deep pass and is then replaced by
  the full set for the same document version. The three independent whole-file
  passes — the analyser walk, the compiler/optimiser checks, and cross-file
  resolution — now run concurrently, and workspace warm-up fans out so a cold
  workspace's first project-wide request hits a warm cache. Settled diagnostics
  are byte-identical to before; only the delivery is faster.
- **Diagnostics persist for closed files.** A file that reported problems keeps
  its **Problems** entry and File Explorer badge after its editor tab is closed,
  recomputed from the file on disk and refreshed when it changes.
- **New `E005` diagnostic — wrong argument-count shape.** Catches an in-range
  argument count that does not fit a command's paired-argument pattern: an odd
  `dict create` / `dict replace` / `dict update` tail, an unpaired `foreach`
  list, or a `switch` whose count matches neither its shorthand nor its
  pattern/body-pair form.
- **A new crop of quick fixes.** `S100` gains a numeric-comparison fix and a
  "suppress with a `noqa` comment" action; `T101` and `T102` (option injection),
  `W003` (per-operator), and `W004` all gain fixes of their own.
- **The release version is stamped into every user-facing binary.**
  `tcl --version`, `f5-query --version`, the language server's `initialize`
  response, and the MCP server's `server_info` now report the real release
  instead of `0.1.0`. A development build reports a `git describe` string, so it
  can never claim to be a release.
- **Editor keyword lists are generated from the command registry.** The
  TextMate grammars for VS Code, JetBrains, and Sublime are now generated and
  drift-gated, which substantially widens the set of commands recognised as
  built-ins — `zlib`, `timerate`, `lseq`, `lpop`, `lremove`, `const`,
  `coroprobe`, `binary`, `clock`, `trace`, and many more.
- **TclOO, coroutines, threads, and bignums in the bytecode VM.** The VM behind
  the `tcl` CLI gains `oo::class` / `oo::define` / `my` / `next` / `self`, TIP
  558 `oo::configurable` properties, `coroutine` / `yield` / `yieldto`, an
  `after` / `vwait` / `update` event loop, a shared-nothing `thread` / `tsv` /
  `tpool` package, and arbitrary-precision integers in `expr` and `incr` instead
  of wrapping at the machine word.

## Improvements

- **Far fewer false positives.** `W001` no longer fires on same-file shadows or
  `{*}`-expanded calls; `W002`'s dialect false positives are gone; `S100` and
  `S101` no longer flag ordering comparisons on non-numeric strings and lists;
  `S110` no longer reports byte-array-transparent `string range` / `index` /
  `reverse` / `trim*` as corruption; `W300` and `W103` stay quiet for a variable
  that provably holds a compile-time literal path; `W241` recognises `throw` and
  `tailcall` as loop exits; and `E001` no longer misfires on `history` or TclOO
  dispatch.
- **More real problems caught.** `T106` detects nested double-encoding;
  `W300`/`W103` flag command-substitution paths such as `source [f]`; `W003`
  gates `expr` operators from the registry, so a symbolic `**` under Tcl 8.4 is
  reported; and the taint codes `T100`–`T102` gained TclOO, namespace, and
  `interp alias` resolution with registry-driven sink classification in place of
  hardcoded sink lists.
- **Tighter highlighting.** Squiggles for `W101`, `W126`, `W212`/`W216`, `W214`,
  `W213`, `W003`, `T100`–`T102`, `S100`/`S102`, and `E200` now narrow to the
  offending token instead of underlining the whole command.
- **Wider arity checking.** TclOO constructor calls, direct `apply` lambdas,
  `after` callbacks, `next`/`nextto`, braced multi-word command prefixes, and
  `namespace ensemble create -command` are all arity-checked now, and an
  `interp alias` deletion followed by a re-declaration is tracked so a call is
  never checked against a stale target.
- **Better recovery from unclosed delimiters.** `E100` and `E200`–`E203`
  recovery now recognises user-defined commands — the document's own procs,
  TclOO classes, aliases, `rename` targets, procs from other indexed workspace
  files, and `package require`d library commands — so a file that calls its own
  procs no longer loses the rest of the document to analysis after a break.
- **Closer parity with C Tcl.** Braced words collapse line continuations
  correctly; `format` handles size modifiers, `%#X`, and `%0Ns` padding;
  `string totitle` genuinely titlecases; `string map -nocase` folds full
  Unicode; `catch` binds a complete error-options dict; `return -level N`
  performs the C countdown; and `expr int()` preserves precision above 2^53.
- **Correct TclOO method reference counts** in the code lens, and
  destructuring-writer variables are typed from the registry rather than from a
  return type.

## Bug Fixes

- **The server's buffer could permanently diverge from the editor's during fast
  typing.** Incremental edits were applied in whatever order the runtime
  scheduled them rather than the order they arrived, and because each edit is a
  range computed against the previous version, one applied out of turn was
  spliced into text it was never computed against. The buffer then stayed wrong
  until the file was closed and reopened: semantic tokens landed on the wrong
  lines and lengths, and hover, completion, and go-to-definition resolved at the
  wrong offsets. Separately, a request could be answered from a buffer that was
  still missing an edit the editor had already sent. Edits are now applied in
  arrival order, and a request always observes every edit that preceded it.
- **Crashes on multi-byte characters.** Unchecked string slicing in the
  analyser and the dialect-directive detector panicked when a UTF-8 character
  boundary fell on a slice offset — an em dash or a curly quote in a comment
  ahead of a keyword was enough to trigger it.
- **A silent miscompile in constant propagation (`O102`).** Load forwarding had
  no trace, aliasing, or barrier safety and could forward a literal past a write
  that changed it. Variable-trace safety is now centralised into the constant
  propagation pass itself, so a trace installed anywhere in the module forces
  the value overdefined.
- **Miscompiles in static proc folding (`O103`).** It ignored reachable
  fall-through exits, never folded procs relying on Tcl's implicit
  "result of the last command" rule, folded through an empty `if` body to the
  wrong value, and folded calls whose target had since been renamed or aliased
  over.
- **Optimiser soundness gaps (`O100`, `O101`, `O102`, `O103`, `O107`).**
  Dialect-blind octal folded `expr {010 + 1}` wrongly under Tcl 8.x and F5
  iRules; folding proceeded through a renamed or shadowed `expr`; a callee's
  `global x; set x …` silently vanished from constant propagation across an
  opaque call; and `uplevel` bodies invalidated aliasing assumptions.
- **`foreach` and `lmap` list splitting** in constant propagation now uses real
  Tcl list semantics — `{a {b c} d}` is three elements, not four.
- **Runtime memory safety.** A use-after-free when a write trace unset the
  variable during `lappend`/`append` (which surfaced as intermittent heap
  corruption), `append x(0)` corrupting the variable store instead of erroring,
  and a leaked intermediate sub-dict on the `dict set`/`unset` error path.
- A dropped closing quote or brace in command-substitution parsing, and
  duplicate `S100`/`S101`/`S102`/`S110` diagnostics.

## Breaking Changes

- **Built-in command colours change.** The `semanticTokenScopes` overrides for
  `function.defaultLibrary`, `operator`, `namespace`, and `decorator` have been
  removed for every dialect. In VS Code these overrides *replace* rather than
  supplement the built-in cross-theme defaults, so any theme without a
  `support.function.tcl` rule rendered `set`, `incr`, `lappend`, and `expr` as
  unstyled plain text. Built-ins now fall back to standard theme defaults, so
  colours will look different — and, for affected themes, correct for the first
  time. `lmap` also moves from built-in-command to control-keyword scope.
- **`W123` (unresolved command) is now on by default.** It was registered
  default-off, contradicting the design decision that the Rust port ships it
  default-on. It is a **Hint**, not a warning, so it is unobtrusive, but you
  will see unresolved-command hints you did not see before. Turn it off with
  `tclLsp.diagnostics.W123`.
- **`xcDiagnostics` is split, and both toggles now genuinely default off.** The
  F5-iRules-only XC100–XC301 translatability lints stay under
  `tclLsp.features.xcDiagnostics`; general-purpose whole-workspace resolution —
  suppressing `W120`/`W123` for a proc defined in another file, and reporting
  `E002`/`E003` for cross-file calls — moves to the new
  `tclLsp.features.crossFileResolution`. Both are now explicitly off by default:
  previously an unset value fell through to a catch-all `true`, silently
  enabling workspace-wide cross-file scanning and the XC lints for anyone who
  had never touched the setting. If you were relying on that implicit
  behaviour, opt in explicitly.
- **`W308` is re-categorised.** The catalogue entry "`subst` without
  `-nocommands`" (security) is retired. `W308` is now documented as "Unknown
  TclOO method" (warning), which is what the compiler has always actually
  emitted. The `tclLsp.diagnostics.W308` setting name is unchanged but now means
  something different from the old documentation.
- **`E200` is redefined** from "Shimmer parse error" to "Unterminated command —
  the parser could not tell where it ends", and `E001` broadens from "missing
  subcommand" to "missing dispatch word", which also covers `$obj` invoked with
  no TclOO method.
- **`scripts/tcltest_sweep/` is removed.** It is replaced by the Rust-native
  `cargo xtask tcltest-sweep` (`make tcltest-sweep`).
</content>
