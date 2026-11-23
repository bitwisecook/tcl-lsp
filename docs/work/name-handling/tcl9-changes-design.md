# Tcl 8.6 to 9.0 and 9.1: who owns the release differences

Which user-visible differences between Tcl 8.6, 9.0 and 9.1 the analyser, the optimiser, the LSP, the VM and the Rust runtime model, and whether each is modelled once. The change list comes from `tmp/tcl9.0.4/changes.md` (9.0.0 to 9.0.4) and `tmp/tcl9.1.0/changes.md`. The 38 fixtures in [`oo-dispatch-probe/tcl9-changes/`](handover/oo-dispatch-probe/tcl9-changes/) each isolate one change and print their results; they were run under tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, the VM (`tclvm --tcl-version 8.6|9.0|9.1`) and the runtime (`run_script --quiet --tcl-version 8.6|9.0|9.1`), each with the matching `TCL_LIBRARY` (`engines.tsv`). The analyser was asked through `mcp__tcl-lsp__analyze` under `tcl8.6`, `tcl9.0` and `tcl9.1` (`analyser-mcp.out`), with navigation, rename and the optimiser probed where a change affects them. The MCP build is cached and may lag `afe2f2d8` slightly.

**Verdict:** availability is handled well. Every new or removed command, subcommand, option, operator and math function in the fixtures is release-gated by the registry surfaces, and the analyser reports W002, W004, W137 or W003 in the wrong release. Changes of *meaning* are handled poorly. Octal literals fold correctly per release, but the analyser has no rule for tilde paths, `tcl_precision`, the removed `binary` encoding, `#` in `expr` before 9.0, writes to a `const`, `lseq` with an expression, `regexp \z` before 9.1, or the TIP 278 variable lookup. The optimiser folds a double that `tcl_precision` changes (a wrong rewrite in 8.6), and references and rename apply 9.0 variable lookup to 8.6 code (a wrong rename). Each engine has wrong answers: no `case` in 8.x mode in either engine, no strict channel decoding in 9.x in either, `scan %Ld` overflow returns empty in both, and in 9.1 mode any `lseq` makes the VM exit silently. The release facts have an owner, `tcl-dialect`, but the analyser reads only part of it.

## Measured

Rows are the changes the fixtures cover; "Analyser" is what `mcp__tcl-lsp__analyze` reports for code that is wrong for the selected release; "VM" and "Runtime" compare each engine with tclsh at the same release. TIP numbers are those cited in `changes.md` or in the code; "—" means none is cited there.

| Change | TIP | Analyser | VM | Runtime |
| --- | --- | --- | --- | --- |
| `0NNN` is decimal, `0o`, `0d` prefixes (t01) | 114 | folding per release (O101: 8.6 `9`, 9.0 `11`); W230 for `lindex … 010` in 9.0; W148 for `0d10` in 8.6; nothing for `08` (an error in 8.6) and no portability note for `010` | ok | ok, except `expr {0d10}` returns `0d10` inside `catch` (also `0o10`, `0x10`: j23) |
| `_` digit separators (t02) | 551 | W148 in 8.6 | ok | ok |
| `~` no longer expanded (t03) | 602 | W002 for `file home`, `file tildeexpand` in 8.6; nothing for `~/x` in 9.0; the `file` hover still says names undergo tilde substitution (`tcl-registry/src/commands/tcl/file_.rs:1007`) | ok | ok |
| `case` removed (t04) | — | W002+W123 in 9.0 | **missing in 8.4–8.6 mode** | **missing in 8.4–8.6 mode** |
| `trace variable`/`vinfo`/`vdelete` removed (t05) | — | W144 (deprecated) in 8.6, W002 in 9.0 | ok | ok |
| `string bytelength` removed (t06) | — | W002 in 9.0 | ok | ok |
| `tcl_precision` ignored in 9.0 (t07) | — | 9.0: W211/W220 only (no "has no effect"); **optimiser in 8.6 rewrites `set tcl_precision 3; puts [expr {1.0/3}]` to print `0.3333333333333333`** (tclsh prints `0.333`) | ok | ok |
| encoding profiles; `identity`, `binary` removed (t08) | 656, 657 | W004 for `-profile`, `-failindex` in 8.6; **false W210 on the `-failindex` variable** in both releases; nothing for `binary` | ok; `identity` absent in 8.6 mode | as VM |
| channels strict by default (t09) | 656 | none (not static) | **reads invalid UTF-8 without error** | as VM |
| `source` reads UTF-8 (t10) | — | none | ok | ok |
| `array default` (t11), `array for` (t12) | 508, 421 | W002 in 8.6 | ok | ok |
| `const`, `info const*` (t13) | 677 | W002 in 8.6; nothing for `set`/`unset` of a constant in 9.0 | ok | ok |
| `lseq`; expressions removed by TIP 746 (t14, t32) | 629, 746 | W002 in 8.6 (W123 suggests `lset`); nothing for `lseq 0 {1+2}` | ok in 9.0; **9.1 mode exits with status 1 and no message on any `lseq`, even inside `catch`** (t38) | ok |
| `string insert`, `string is dict` (t15) | 504 | W002, W137 | ok | ok |
| `lpop`, `lremove`, `ledit` (t16) | — | W002+W123 in 8.6 | no `lremove` | ok |
| `dict getwithdefault`/`getdef` (t17) | 342 | W002 | ok | ok |
| unqualified variable in `namespace eval` no longer finds the global (t18) | 278 | no diagnostic needed; **references and rename use the 9.0 rule in 8.6**: renaming `::x` leaves `namespace eval ns {set x …}` behind, which then writes `ns::x` | ok | ok |
| `lt`/`gt`/`le`/`ge` (t19) | 461 | W003 with a rewrite | ok | ok |
| `#` comments in `expr` (t20, comments c18) | 582 | **nothing in 8.6** (an error there) | ok | ok |
| `isinf`, `isnan`, `fpclassify`, `int()` unbounded (t21); C99 functions in 9.1 (t37) | 521, 745 | W002 per release | ok; `cbrt(27.0)` is `3.0` in 9.1 mode, tclsh `3.0000000000000004` | no `fpclassify`; `cbrt` as VM |
| `%d`/`%x` truncate to 32 bits (t22) | — | none; the folder declines (safe) | ok | ok |
| legacy `puts ch str nonewline` removed (t23) | — | E003 in 9.0 (right) and **in 8.6 (false)** | **rejected in 8.x mode** | as VM |
| new command roster (t24, t25, t27, t33, t36) | 426, 430, 462 | W002 for each in 8.6 | no `zipfs`, `tcl::process`, `oo::singleton`, `chan`; no `inject` in 8.6 mode | no `zipfs`, `tcl::process`, `fpclassify`, `oo::define::classmethod`; `oo::define::private` present in 8.6 mode; `chan isbinary` refused |
| `lsearch -stride`, `regsub -command` (t26) | — | W004 in 8.6 | ok | ok |
| 9.1 `subst -variables`, `switch -integer` (t28, t29) | 712, 730 | W004 in 9.0 | ok | ok |
| 9.1 `lfilter`, `unicode` (t30) | 735, 726 | W002 in 9.0 | **missing in 9.1 mode** | **missing in 9.1 mode** |
| 9.1 `regexp \z` (t31) | — | **nothing in 9.0**, although `TclVersion::regex_z_anchor` exists | ok | ok |
| `scan %Ld`/`%ld` beyond 64 bits (t34) | — | none | **returns empty** in every mode (tclsh 8.6: clamps to `9223372036854775807`; 9.0: the bignum) | as VM |

Counted across the 38 fixtures and three releases each, 60 engine runs differ from tclsh; after removing differing error texts (`?channelId?` against 9.0's `?channel?`) and the `chan`/zipfs roster gaps, the wrong answers are the bold cells above.

## Census

**Release facts.** `tcl-dialect` is the owner, in four places:

- `TclVersion` methods (`tcl-dialect/src/version/release_behaviour.rs`), about 40 of them, including `namespace_var_global_fallback` (:35) and `regex_z_anchor` (:63);
- `DialectProfile` policies (`tcl-dialect/src/profile/engine_policy.rs`), including a second `namespace_var_global_fallback` (:35) that keys on `runtime_base` and answers `false` for a profile with no base, where the `TclVersion` one has no such case;
- the lexer grammar (`tcl-dialect/src/grammar.rs`): `NumberSyntax` (:503) for octal, `0d` and `_`; `precision_variable` (:739) and `with_precision_written` (:909); `ExprCommentStyle` (:244); `BraceBackslashNewline` (:983);
- command availability in the registry surfaces (`surface`/`available` on each spec), which drive W002, W004 and W137.

**Consumers that read them:**

- Engines. The runtime sets `ns_var_global_fallback` from the version (`runtime/rust/src/interp/setup/construction.rs:291`, `namespace.rs:575`), and the VM reads it in `tcl-vm/src/interp/variables/resolution.rs:32`. `tcl-cmd-core/src/channel.rs:319` and `:347` choose a strict profile for 9.0 channels, but the read path decodes leniently: `decode_text` uses `String::from_utf8_lossy` (`channel.rs:129`), so the profile is recorded and never enforced on input (inferred from the code and t09).
- Analysis.
  - `var_resolve/scalar_binding.rs:194` and `var_resolve/original_bytes.rs:627` read the TIP 278 policy, and so does `tcl-lsp-core/src/definition/variables.rs:510`, for definitions only. References and rename gave identical answers under 8.6 and 9.0 (t18), so their path does not reach it (inferred).
  - Precision has an owner in the value-transfer path: `module_may_set_precision` (`command_binding/mutation_summary.rs:500`), `precision_written` (`value_transfer.rs:628`, `tcl-registry/src/value_transfer/context.rs:134`), and `tcl_expr_eval/services.rs:235` applies it. The O101/O100 path that produced the wrong 8.6 rewrite did not decline (measured; whether it bypasses `precision_written` or the MCP build predates it is not checked).
  - `regex_z_anchor` has no analysis consumer (*read*).
- Engine gaps. No `case` in `tcl-vm`, `runtime/rust` or `tcl-cmd-core`, although the registry declares it for 8.4–8.6 (`tcl-registry/src/commands/tcl/case_.rs:121`). No `lfilter`, `unicode`, `timer`, `zipfs`, `tcl::process`, or `lremove` in the VM.

**Duplicates:**

- two TIP 278 policies (`TclVersion` and `DialectProfile`) that disagree on profiles without a base;
- two precision gates: value transfer, and whatever O101 uses;
- release-blind hover prose: `file_.rs:1007` against the release-aware `filename.rs:63`;
- the expression-literal bug, which appears in the runtime only (t01, j23).

## Design

Release differences come in three kinds, and each needs a different owner:

1. **Availability** (a command, option, operator or function exists or not). This is already one owner: registry surfaces. It works.
2. **Meaning** (the same text means something else: octal, tilde, precision, TIP 278, expression comments, `\z`, `%d` width, channel profile, `source` encoding). These belong in `tcl-dialect` as named release facts. They must be read by binding, SCCP, value transfer, navigation and the engines through one accessor each, and never by comparing versions (fault 4 of the [cross-cutting review](cross-cutting-review.md)). The two TIP 278 functions become one. The grammar half (`NumberSyntax`, `ExprCommentStyle`) is already right and should be the model for the rest.
3. **Effects** of new commands (`const` makes a read-only variable, `array default` makes reads of missing keys succeed, `lpop`/`ledit`/`lremove` write their variable, `-failindex` writes a variable). These are registry effect descriptors (decision 6 of [design integration](design-integration.md); fault 9). W210, the frame-effect summaries and the name world's variable state should read them. A `const` cell is a `VariableTarget::Cell` with a read-only flag in the per-point name world, so a write to it is a proven error, not a guess.

The potential-execution owner (decision 8) supplies the whole-program facts: "no `tcl_precision` write" and "no `tcl::mathfunc` override". Folding a double then needs a receipt that names that fact (decision 7), and a module that may write `tcl_precision` gets `Answer::Unknown` with a reason. The engines should expose the same release facts through the engine traits, so the VM and the runtime cannot drift.

Conformance: `engines.tsv` is a vector table. Its rows become conformance-matrix rows (fault 5) with tclsh 8.4–9.1, the VM, the runtime and the analyser as columns, and a release-fact lint checks every new `tcl-dialect` fact against a fixture row.

## Plan

- **Fix the engine wrong answers.** Add `case` to `tcl-cmd-core`, registered in 8.4–8.6 mode in `tcl-vm` and `runtime/rust`, and accept the legacy `puts ch str nonewline` form there. Make channel input honour `EncodingProfile::Strict` (`tcl-cmd-core/src/channel.rs`, replacing the lossy `decode_text` on the read path). Clamp or return the bignum for overflowing `scan %ld/%Ld` per release (`tcl-cmd-core` scan). Fix the runtime's `expr` returning the literal spelling of `0o`/`0d`/`0x` constants inside `catch`. Find and fix the VM 9.1 silent exit on `lseq`. Done when t04, t09, t23, t34, t38 and j23 match tclsh in `engines.tsv`, as end-to-end tests in both engines.
- **Fill the engine rosters or record non-goals.** For `lremove`, `chan` (VM), `zipfs`, `tcl::process`, `fpclassify` (runtime), `oo::singleton` (VM), `oo::define::classmethod` (runtime), `lfilter`, `unicode`, `timer` (9.1), and runtime 8.6's stray `oo::define::private`, either implement them in `tcl-cmd-core` behind the engine traits or list them in an explicit non-goal table that the conformance matrix reads. Owners: `tcl-vm`, `runtime/rust`, `tcl-cmd-core`.
- **One TIP 278 fact.** Delete `DialectProfile::namespace_var_global_fallback` or make it delegate to `TclVersion`, with an explicit answer for profiles without a base. Route references, rename and the name world's variable lookup through it, so t18 under 8.6 links `namespace eval ns {set x …}` to `::x`. Done when `find_references` and `rename` differ between 8.6 and 9.0 on t18 as tclsh does.
- **Precision as a whole-program fact.** Make O100/O101 and every double-spelling fold consult `precision_written` (`command_binding/mutation_summary.rs`), later the potential-execution owner, and decline with a reason. In 9.0 replace W211/W220 on `tcl_precision` with a release note that it has no effect. Done when t07 under 8.6 is not rewritten.
- **Meaning-change diagnostics from release facts.** Each of these reads a `tcl-dialect` fact, not a version comparison:
  - a leading-zero literal whose value differs between the target releases, and `08` in 8.x (`NumberSyntax`);
  - `~` at the start of a path argument in 9.0 (a value-grammar role on path arguments);
  - `#` inside `expr` before 9.0 (`ExprCommentStyle`; today the expression parse simply raises nothing);
  - `\z` before 9.1 (`regex_z_anchor`);
  - `lseq` with a non-numeric operand (TIP 746);
  - `encoding … binary` and `identity` in 9.0.

  Owners: `tcl-compiler` diagnostics and the registry value grammars. Done when t01, t03, t08, t14, t20 and t31 each get one diagnostic in the release where they fail and none in the other.
- **Effect descriptors for the new mutators.** Declare in the registry that `const` makes a read-only cell, that `array default` changes missing-key reads, that `lpop`/`ledit`/`lremove` and `encoding -failindex` write their variable argument, and that `info constant` reads. Then W210 stops firing on t08 and a write to a constant is reported. Owner: `tcl-registry` effect data (decision 6).
- **Release-aware hover prose.** Make `file`'s hover (`tcl-registry/src/commands/tcl/file_.rs:1007`) and any other hover that states a release-specific behaviour come from a release-keyed snippet, as `filename.rs:63` already does.
- **Conformance rows.** Move `oo-dispatch-probe/tcl9-changes/` into the conformance matrix (fault 5) with columns tclsh 8.4–9.1, the VM, the runtime, the analyser's codes per dialect and the optimiser's rewrite. Add a lint that every `TclVersion`/`DialectProfile` release fact is named by at least one row.

## Status on the current source

Not reproduced on the current source: the 8.6 `tcl_precision` rewrite. Navigation and rename rows were not re-run. Details: [audit summary](tcl-areas-audit.md) and [`tcl-areas-head/`](handover/oo-dispatch-probe/tcl-areas-head/). Plan items 153–199.
