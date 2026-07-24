# v2.1.14

**2.x alpha — pre-release channel.**

Another pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel or the JetBrains Marketplace **eap**
channel, or download the pre-release VSIX / plugin / native binaries from this
GitHub release. The stable **1.x** line stays the default for everyone who has
not opted into pre-releases, and a `2.1.x` build never becomes the "latest"
GitHub release or the default Marketplace download.

This release is dominated by a large per-command audit campaign that
cross-checks the registry backing hover, completion, and diagnostics against
the real Tcl 8.4-9.1 manpages (and the iRules/Expect/EDA dialect surfaces)
command by command, plus a further round of the issue #923 differential-audit
campaign (tclsh-vs-analyser correctness fixes). Alongside that: a crash fix
for deeply nested Tcl source, several false-positive/false-negative
diagnostic fixes, and a run of TclOO reference-finding and code-lens fixes.

## New Features

- **Two new diagnostics: W141 and W142.** W141 validates the *content* of an
  option's value where arity alone can't express the constraint (e.g.
  `return -errorstack`'s list must have even length). W142 flags a
  context-gated restriction — currently `return`'s bare-only form inside an
  iRules `when EVENT { }` body.
- **`tcl::mathop` recognises `lt`/`le`/`gt`/`ge`.** These Tcl 9.0+ operators
  (TIP 461) were missing from the registry entirely; the mathop spec also
  drops three entries (`&&`, `||`, `@`) that were never real
  `::tcl::mathop` members in any released Tcl.

## Improvements

- **~150 of 275 core Tcl commands re-audited against the real manpages**
  (8.4 through 9.1) and the iRules/Expect/EDA dialect directories, correcting
  version gates, options, arity, side effects, and hover text wherever the
  registry disagreed with the documented (or empirically-verified) behaviour
  — `open`, `binary`, `after`, `array`, `catch`, `coroinject`, `format`,
  `namespace`, `interp`, `glob`, `string`, the whole TclOO family, and many
  more.
- **iRules/Expect/EDA dialect gating is now fully explicit.** The old
  subtractive `IRULES_DISABLED_COMMANDS` ban-list is gone; every command
  declares the dialects it actually belongs to instead of defaulting to
  "universal". This closes real leaks — about 72 stdlib package commands
  (`http::*`, `msgcat::*`, `safe::*`, `platform::*`) and a few sandbox-
  unreachable ones (`puts`, `read`, `parray`, …) were previously visible in
  iRules even though the sandbox can't actually reach them — while keeping
  every genuine core command available. `::tcl::` namespace commands
  (Tcl 8.5+) are correctly excluded from 8.4/iRules, and Expect's
  `exit -onexit`/`-noexit` and iRules' `event` command no longer leak into
  dialects that don't have them.
- **Further issue #923 differential-audit fixes** (follow-up to #963):
  more tclsh-vs-analyser correctness work across rename, workspace
  indexing, and reference resolution, plus expanded command specs for
  `dict`, `prefix`, `oo_link`, `tcl::optproc`, `[incr Tcl]` class linkage,
  and Snit types/widgets/widgetadaptors.

## Bug Fixes

- **Fixed a crash on deeply nested Tcl source.** The analyser could
  `SIGABRT` the whole process on control flow nested ~100-150 levels deep —
  the recursion-depth guard was correct, but the worker thread's default
  2 MiB stack wasn't big enough to hold that many real stack frames.
  Recursive analysis now runs with a larger stack.
- **W123 "unknown command" is now accurate in both directions.** A proc,
  class, alias, or `rename`d command that was later deleted or renamed away
  without being re-established now correctly *does* draw W123 everywhere the
  check runs (previously only some resolution passes saw the deletion, so
  the diagnostic was inconsistently suppressed). Separately, built-in `expr`
  math functions like `sin($x)` / `max($a, $b)` no longer draw a spurious
  W123 — they were never unknown to begin with.
- **Fixed an unsound constant-folding false positive (I230).** An
  interprocedural parameter-constant analysis trusted a proc parameter as
  compile-time-constant whenever every *resolvable* call site agreed, even
  when a real call site went unresolved and silently dropped out of
  consideration — so a genuinely-alternating parity check could be flagged
  "always false".
- **W129 safe-interp hidden-command detection now sees through bracket
  substitution.** A hidden command reached only via `[...]` — most notably
  the common `[list apply {...} $x]` deferred-command idiom used in
  `package ifneeded`, `trace add ... command`, and `after idle` — went
  unflagged before.
- **`apply` lambda highlighting works through `[list apply {...} $x]`
  indirection**, fixing the common `package ifneeded name ver
  [list apply {dir {...}} $dir]` pattern that a narrower earlier fix
  missed.
- **TclOO reference-finding and code-lens fixes:**
  - Property, constructor, and destructor members now get a
    reference-count code lens (previously only methods did).
  - Method and classmethod code lenses are clickable again, and
    classmethod reference counting is correct.
  - Method-dispatch resolution (`my`, `next`, `nextto`, `$obj method`) for
    Find References, rename, and call hierarchy now works when the call is
    nested inside `if`/`while`/`foreach`/`switch`/`try`/`catch`/`eval`/
    `dict for` at any depth, not just at the top level.
- **`return`'s new W142 context gate no longer misfires inside a `proc`
  that is lexically written inside a `when` block** — the restriction is
  meant only for code directly in the event body, not code inside a
  (separately-flagged) misplaced proc.
- **Taint analysis (T100) no longer flags `apply`'s bound parameters or
  `open`'s access-mode argument as code-injection sinks** — only the
  actual lambda body / pipeline-selecting filename is a real sink for
  either command.
