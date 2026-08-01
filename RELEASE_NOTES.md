# v2.1.15

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

This release is dominated by a deep TclOO correctness pass — class-side versus
instance-side member visibility, `classmethod` dispatch, `renamemethod`,
MRO/mixin handling, and the `[self class]`/`info exists` constant-folding
chains all got hardened against real-world TclOO code, closing a long run of
false positives and false negatives in diagnostics, navigation and rename. The
second major thread is a corpus-driven performance and memory-leak sweep that
found and fixed several unbounded per-file caches and quadratic cross-file
passes on large workspaces, cutting both steady-state memory and typing/
navigation latency substantially. Alongside those: `namespace import`/`export`
gained a proper event-ordered lifecycle model, package version selection was
rewritten against a `tclPkg.c`-faithful comparator, and a run of infrastructure
work landed a wasmtime bump for two RUSTSEC advisories plus a new
command-registry "spec studio" editor.

## New Features

- New diagnostic **W143** flags direct calls into private `::tcl::` implementation namespaces (e.g. `::tcl::dict::create`), with a quick fix that rewrites the call to its public form when one exists and the command head isn't quoted or braced.
- New diagnostic **W315** flags a TclOO class or `oo::define` body that would abort at runtime — retracting a member absent from that side, or renaming onto a name that side already holds — matching cases where the interpreter creates no class at all.
- Namespace names are now navigable symbols: go-to-definition, hover and find-references now work on a namespace argument (`namespace children`, `exists`, `delete`, `upvar`, `eval`), locally and across files.
- iRules `when` event handlers now appear in the outline, breadcrumbs, Cmd+Shift+O and workspace symbols, with the statements inside each handler correctly nested underneath it instead of listed as siblings.
- Sticky scroll now defaults to the folding-range model for the non-version-pinned Tcl code languages (`tcl`, `tcl-irule`, etc. — VS Code's dotted `tcl8.4`/`tcl8.5`/`tcl9.0`/`tcl9.1` ids can't carry this override and keep the definitions-only outline model), fixing the case where non-proc-heavy files had no sticky scroll at all, and BIG-IP `.conf` files gained real per-stanza folding, including inside embedded `ltm rule` bodies.
- `renamemethod`'s destination is now a first-class navigable member (definition, hover, rename), rather than disappearing from the outline once renamed.
- Call hierarchy, code lenses and find-references now recognise `classmethod` dispatch (`Factory make`) and agree with each other on both the click target and the reference count, where previously each answered from a different, narrower scan.
- `expr` math functions (`sin(...)`, `abs(...)`, etc.) now get hover and completion, and go-to-definition when resolved to a user-defined `::tcl::mathfunc` override — using the same two-candidate precedence C Tcl itself uses; ordinary calls to built-ins still have no source location to jump to.

## Improvements

- A large memory-leak and performance sweep, driven by profiling a tcllib-sized workspace: closed-file diagnostics and badge caches are now bounded and evicted instead of growing without limit, closing or renaming a file no longer leaks its underlying analysis state, several workspace-wide views (defined-command names, command-link maps, cross-file call-site evidence) are now memoised instead of rebuilt on every request, the initial workspace scan runs in parallel and no longer performs a full deep analysis of files nobody has opened, and watched-file batches (e.g. a branch switch) reindex in one batched pass instead of one file at a time. Together these fixed a real leak that could reach several gigabytes of RSS and an eventual OOM on large workspaces, and cut per-keystroke and per-request latency markedly.
- `workspace/symbol` (Ctrl+T) is now answered from the workspace index instead of re-analysing every open document on each keystroke, and as a result now also finds symbols in files the workspace has indexed but the user never opened.
- Semantic-token convergence (serving coarse tokens quickly, then refreshing with enriched ones) no longer discards a pending refresh under load or loops indefinitely, and no longer triggers a full-workspace deep analysis just to answer tokens for one file.
- `namespace import`/`export` is now modelled as an ordered lifecycle — install, `namespace forget`, `-force` shadowing, source rename/delete/redefinition, and chains through a re-exporting namespace — instead of a static end-of-file snapshot, fixing a range of cases where navigation followed an alias that had already been revoked, or missed one that had just been installed.
- Package version selection (`package require` navigation and hover) now uses a comparator ported line-for-line from Tcl's own `tclPkg.c`: `-exact` is honoured, the highest satisfying and stable release is preferred over "first discovered", and version components of arbitrary size compare correctly instead of silently overflowing.
- Brace-quoted variable names (`set {$n} v`) are now keyed by their literal written spelling across hover, definition, rename, references and semantic tokens, instead of being normalised as if the `$` and braces had been substituted.
- TclOO member resolution now correctly distinguishes the class-object ("class-side") side from the instance side for methods, filters and export lists, resolves a class-side receiver through the same lexical/import rules ordinary command lookup uses, and reaches classmethod calls made from files that only consume the class rather than define it.
- wasmtime was bumped to 46.0.2 to pick up fixes for two RUSTSEC advisories; other Rust workspace dependencies were also updated to their latest compatible versions.

## Bug Fixes

- `info exists` no longer folds to a constant inside a TclOO method body when the name is an instance variable (fixing 17+ false positives in real-world TclOO code in one file alone), and a method parameter that shadows an instance variable now folds correctly instead of always false.
- Memoised compiler-check diagnostics now run the full check family over TclOO methods and other non-procedure body units, matching the uncached path — a large TclOO file was previously missing around 20 optimisation hints, but only when served from the warm cache.
- `uplevel #0` bodies now resolve bare command calls against the global namespace instead of the enclosing namespace, fixing false liveness/dead-code conclusions for calls made through it.
- A constructor call is no longer typed against a class that has since been renamed or deleted, and the "was this proc already reachable" liveness check now follows the whole call graph instead of one level of indirection — both were sources of false-positive diagnostics on live code.
- Fixed the `namespace inscope` bytecode VM behaviour, which was space-joining its trailing arguments instead of appending them as list elements, matching `Tcl_ConcatObj` semantics.
- A single-bit collision in the internal command-trait table (`SAFE_INTERP_HIDDEN` and `TRANSFERS_CONTROL` shared one flag) caused a false W129 on `break`/`continue`/`yield`/`yieldto`/`tailcall` inside safe-interpreter code, and silently withheld the "Inline proc" code action on ten unrelated commands (`file`, `exec`, `open`, `socket`, and others); both are now fixed.
- The `mathop` operators (`+`, `eq`, and the rest) are now marked pure, so the optimiser can hoist and common-subexpression-eliminate them like any other pure expression.
- Fixed a startup race where `workspace/symbol` could answer from an empty index before the workspace scan had started, and a race where dialect re-detection missed an edit that completed a version-guard or dialect marker deep in a file.
- Cross-document TclOO rename now rewrites pure-consumer call sites, not just overriding or inheriting classes, and an unwrapped `deletemethod`/`renamemethod` (outside a `self`/`private` wrapper) now actually removes or moves the member in navigation and completion instead of leaving a stale entry behind.
- Rename now refuses an edit that would abort a TclOO class definition — renaming a method onto a name already live on that side, or onto its own retracted source — instead of silently producing broken `oo::define` bodies.
- `namespace export -clear` after an import no longer retroactively revokes an alias the import already installed, exporting a name after an import no longer retroactively grants it, and only the first leading `-clear`/`-force` option word on a line is now honoured, matching the interpreter.
