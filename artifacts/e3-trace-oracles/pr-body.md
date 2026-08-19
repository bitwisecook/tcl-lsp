Fixes #1438
Fixes #1440
Fixes #1444
Fixes #1629

## Summary

- **#1438** — a write-trace error no longer un-stores the value in the bytecode
  VM; C commits the write before calling the traces and never restores it.
- **#1440** — per-kind firing order across both engines: newest-first for
  variable, command, and execution-enter traces; whole-array traces grouped
  before element ones; `trace remove` deletes the newest match; namespace
  teardown fires each entity's list contiguously.
- **#1444** — the deprecated 8.x `trace variable` / `vdelete` / `vinfo` forms
  route through the shared parser in both engines and are gated by the
  registry's release model, so they exist at 8.x and are `bad option` at 9.0+.
- **#1629** — one out-of-scope base repair, called out below and safe to drop.

Three trace defects across both engines, plus the shared argument-decoding
owner they all route through. Every behaviour below is pinned byte-for-byte
against `tmp/tcl8616-install/bin/tclsh8.6` (**8.6.16**) and `tclsh9.0`
(**9.0.4**), with the governing C function cited at each site.

## #1438 — a write-trace error must not un-store the value

`TclPtrSetVarIdx` (`tclVar.c` 9.0.4:1913) swaps the new value into the cell at
:2023-2032, *then* calls the traces at :2040-2046, and on a trace error jumps
straight to `cleanup:` (:2070), which never restores the old value. 8.6.16 has
the same shape (`cleanup:` at :2026).

`runtime/rust` was already correct. The VM rolled back, and its doc comments
asserted that rollback as intended. Removed from `set_var` and
`set_array_elem`, so `set` / `append` / `incr` / `lappend` all keep the written
value — on a scalar, on an array element, and on a cell the failing write
itself created.

## #1440 — per-kind firing order

C prepends every registration (`TraceVarEx` `tclTrace.c` 9.0.4:3090-3092,
`Tcl_TraceCommand` :1016-1018) and each firing loop walks the list head to
tail, so the **newest** fires first. The single exception is the explicit
reverse scan for `leave` and `leavestep` (:1333-1347).

- **VM**: an array-element access now runs the containing array's traces as a
  group before the element's own, whichever was registered first
  (`TclCallVarTraces`, the `arrayPtr` loop at :2581 preceding the `varPtr` loop
  at :2623). Command `rename` and `delete` traces fire newest-first
  (`CallCommandTraces`, `tclBasic.c`:3972-3974).
- **runtime**: variable traces fire newest-first for every op and gain the same
  array-before-element grouping; command traces likewise.
- **Namespace teardown** fires each entity's trace list *contiguously*,
  newest-first within the entity — see the adversary round below.
- `trace remove` (either spelling) deletes the **newest** of several identical
  registrations, since C breaks at the first match walking head to tail. All
  four removal sites search from the newest end. Observable twice over: in the
  survivors' firing order and in `trace info`.
- Incidental fixed alongside: `trace info` rendered its op set in the
  `opStrings` table order. C's three `TRACE_INFO` arms hard-code a *different*
  sequence — `array read write unset`, and `rename delete` — so the shared
  parser now returns that canonical order and both engines render correctly by
  construction.

## #1444 — the deprecated 8.x `trace variable` / `vdelete` / `vinfo`

C compiles them behind `#ifndef TCL_REMOVE_OBSOLETE_TRACES`
(`tclTrace.c` 8.6.16:198-206); 9.0 dropped them (9.0.4:196-201).

- Both engines resolve `trace`'s option word against the option set the
  **registry** declares for the pinned release, so the `DialectSet::TCL8X` gate
  on those three subcommands is the single place the 9.0 boundary lives. The
  forms work at 8.x and report `bad option "variable": must be add, info, or
  remove` at 9.0+, and the enumeration for any other bad option follows the
  same set. Prefix resolution (`trace var ...`) comes with it.
- The VM's permissive letter filter is replaced by the shared
  `parse_legacy_variable_ops`: `rwua`-only validation with C's error text,
  duplicate letters collapsed, and a canonical set that `trace remove variable`
  matches (and vice versa, as C masks `TCL_TRACE_OLD_STYLE` out of the match).
- `runtime/rust` gains all three forms, sharing the modern add/remove body
  exactly as C does by rewriting them.
- A trace installed the legacy way receives the single `rwua` **letter** in its
  callback rather than the operation name (`TraceVarProc` 8.6.16:2002-2011) —
  the one place the legacy form is not merely a spelling of
  `trace add variable`.

## Owner and contract

`tcl_cmd_core::trace` becomes the sole owner of `trace`'s argument decoding:
`resolve_option`, `TraceKind::info_order`, `parse_ops`,
`parse_legacy_variable_ops`, `legacy_ops_letters`, `callback_op_word`. Added to
the owner-resolution manifest and its bullet in
`shared-utility-contracts-rust.md`; `cargo xtask owner-resolution` passes.

The trace contract and the as-built doc now record the group ordering, the
fixed per-kind `trace info` order, the newest-first removal rule, the old-style
callback letter, the 9.0 boundary, and that a failed write keeps both the value
and any cell it created.

## Adversary round

**Substantive finding — per-entity grouping.** Namespace teardown collects
victims into one flat, registration-ordered list, and reversing that list
scrambles entities together. C tears a namespace down one entity at a time (a
per-Var loop for variables, a per-Command loop for commands), completing each
entity's whole trace list before the next, so an entity's callbacks are
contiguous. Interleaved registrations discriminate, and the earlier tests only
used non-interleaved shapes:

| registration order | tclsh 8.6.16 and 9.0.4 | flat reverse |
|---|---|---|
| vars `A1 B1 A2 B2` | `A2 A1 B2 B1` | `B2 A2 B1 A1` |
| cmds `X1 Y1 X2 Y2` | `X2 X1 Y2 Y1` | `Y2 X2 Y1 X1` |
| three entities, twice each | `p2 p1 q2 q1 r2 r1` | interleaved |

`group_newest_first_per_entity` groups by entity key, keeps first-seen key
order, and reverses within each group. Three regression vectors were added
(interleaved variables, interleaved commands, and a teardown whose callback
re-enters the interpreter), and both grouping sites are mutation-verified.

The prior comment conflated two separate claims, now stated apart: *which*
entity fires first is C's hash-table walk and remains deliberately unpinned,
whereas a group being contiguous is pinned regardless of hash order.

**Panic-proofing.** In both engines' `trace` dispatch, a registry-declared
option with no arm in that engine now falls through to `bad option` instead of
`unreachable!()`. The option table is registry *data*, so a data-only spec edit
— a new subcommand or alias — would otherwise become a panic in a shipped
interpreter.

## Review round (Codex, two P2s)

**The old-style letter was lost through namespace teardown.** Teardown reduces
its collected callbacks to a name/prefix pair, dropping the flag, then always
appended `unset` — so an 8.x `trace variable ::ns::x u cb` fired by
`namespace delete` called back with `unset` where tclsh 8.6.16 says `u`. The
explicit-unset path was already correct, so the two disagreed with each other.
This is the single path where #1444's letter convention meets the #1440
teardown work, and neither side's tests covered it.

| case | tclsh 8.6.16 | before |
|---|---|---|
| legacy, fired by `namespace delete` | `L:u` | `L:unset` |
| modern, same teardown | `M:unset` | `M:unset` |
| both on one variable, newest-first | `Mz:unset Lz:u` | `Mz:unset Lz:unset` |
| legacy on an array element | `A:u` | `A:unset` |
| legacy, fired by explicit `unset` (control) | `E:u` | `E:u` |

The collected-callback type splits in two: the variable one carries the flag
and its firing site runs the op word through `callback_op_word`; the command
one does not, because all five `TCL_TRACE_OLD_STYLE` uses in `tclTrace.c` are
in the variable machinery and no command-trace equivalent exists. The grouping
helper became generic over the payload so both shapes still share it. Vectors
cover all four 8.x rows plus the 9.x side, where the legacy form does not exist
and teardown must still report `unset`.

**The `vinfo` spec detail promised the wrong result shape.** It still described
the modern `{opList command}` output; for 8.x the first element is the `rwua`
letter string. That text feeds hover, signature help, and completion, so it was
handing editors a false contract. Reworded to draw the distinction explicitly,
including the fixed r, w, u, a order — which is genuinely different from the
modern arm's `array read write unset`, and is the trap behind the original bug.

## Mutation verification — 26 of 26 killed

Every guard this branch introduces was flipped in turn on the final tree and
confirmed to break at least one named test. The harness refuses to score a
mutant whose anchor did not apply, so a silently-unapplied mutation cannot be
mistaken for a survivor.

| Guard mutated | Killed by |
|---|---|
| `info_order`, variable (to table order) | `operations_are_a_canonical_set_in_info_order` |
| `info_order`, command | `operations_are_a_canonical_set_in_info_order` |
| legacy ops not canonicalised | `legacy_variable_operations_are_flags_not_a_list` |
| `rwua` letter order reversed | `legacy_letters_render_in_rwua_order` |
| `callback_op_word` ignores old-style | `old_style_callbacks_get_the_letter` |
| option word exact-only, not abbreviating | `option_word_resolves_against_the_release_option_set` |
| VM `set_var` rolls back | `write_trace_error_keeps_the_stored_value` |
| VM `set_array_elem` rolls back | `write_trace_error_keeps_the_stored_value` |
| VM element before array | `vm_matches_the_pinned_trace_vectors` |
| VM var traces oldest-first | `vm_matches_the_pinned_trace_vectors` |
| VM cmd traces oldest-first | `vm_matches_the_pinned_trace_vectors` |
| VM remove-var takes oldest | `vm_matches_the_pinned_trace_vectors` |
| VM remove-cmd takes oldest | `vm_matches_the_pinned_trace_vectors` |
| VM option gate open at 9.0 | `legacy_variable_trace_forms_follow_the_selected_release` |
| runtime var traces oldest-first | `trace_remove_drops_the_newest_duplicate` |
| runtime element before array | `whole_array_traces_fire_before_element_traces` |
| runtime `callback_op_word` ignores old-style | `legacy_variable_trace_forms_match_c` |
| runtime cmd traces oldest-first | `command_traces_fire_newest_first` |
| runtime remove-var takes oldest | `trace_remove_drops_the_newest_duplicate` |
| runtime remove-cmd takes oldest | `trace_remove_drops_the_newest_duplicate` |
| runtime ns-unset not reversed | `namespace_teardown_fires_unset_traces_newest_first` |
| runtime ns-cmd not reversed | `namespace_teardown_fires_command_delete_traces_newest_first` |
| runtime option gate open at 9.0 | `legacy_variable_trace_forms_follow_the_release` |
| runtime ns-unset flat reverse | `namespace_teardown_grouping_survives_a_re_entrant_callback` |
| runtime ns-cmd flat reverse | `namespace_teardown_groups_interleaved_command_traces` |
| runtime teardown drops the old-style flag | `legacy_letter_survives_namespace_teardown` |

The ns-cmd-not-reversed guard was a **survivor** on first run: the code was
right but nothing pinned it. C was consulted before assuming so, then a test
was added and the mutant re-run to confirm the kill.

## One out-of-scope commit

`fix(vm-wasm): thread the dialect profile through the satellite compile
service` is **not this lane's work** and is safe to drop or move. `tcl-vm-wasm`
is a separate workspace, so a check over the main one never builds it; #1621
changed two lowering entry points to take the profile instead of its name and
updated every call site the main workspace compiles, missing this crate. The
result is that `cargo test -p tcl-vm --test wasm_coro_e2e` is red on `rust`
itself. The change is the same two-line mechanical update applied everywhere
else. Filed as #1629.

## Verification

Regression vectors live in the tclsh-diffed tables, so they are re-checked
against the real interpreters on every run:
`rust/tcl-vm/tests/command_traces_e2e.rs`,
`rust/tcl-vm/tests/legacy_variable_traces_e2e.rs` (cross-version 8.4 to 9.1),
`rust/tcl-vm/tests/builtins_e2e.rs`, plus unit tests in
`runtime/rust/src/cmd_trace.rs` and `rust/tcl-cmd-core/src/trace.rs`.

`TCL_LSP_TCLSH86` was pinned to the 8.6.16 build throughout, and the
tclsh-diffed tests were confirmed by name to have run rather than skipped —
`/usr/bin/tclsh8.6` on a stock box is 8.6.14, and those suites fall back to it
silently.

## Validation

- [x] I ran relevant tests/checks locally and listed the exact commands.

```
cargo fmt --all --check
cargo clippy -p tcl-vm -p tcl-cmd-core -p tcl-registry --all-targets
TCL_LSP_TCLSH86=tmp/tcl8616-install/bin/tclsh8.6 \
  cargo test -p tcl-vm -p tcl-cmd-core -p tcl-registry --no-fail-fast
cd runtime/rust && cargo fmt --all --check && cargo clippy --all-targets && cargo test --no-fail-fast
make xtask-check
```

Results on the current head: format check clean; clippy clean on every crate
touched; 1967 passed / 0 failed over 61 binaries for `tcl-vm` plus
`tcl-cmd-core` plus `tcl-registry`; 536 passed / 0 failed for `runtime/rust`;
every `xtask-check` drift gate green with no generated-file churn. The
`runtime/rust` format check also reports four files this branch does not touch
— that is the pre-existing #1623 drift on `rust`, left alone deliberately.

## Compiler / diagnostics checklist

- [x] Did this change alter a compiler fact contract? **Yes** — the variable-trace
      dispatch and introspection contract, and the shared owner map.
- [x] Updated the owning design docs:
      `docs/design/contracts/variable-trace-dispatch-and-introspection.md`,
      `docs/design/contracts/shared-utility-contracts-rust.md` (new owner row and
      bullet for `tcl-cmd-core::trace`), and
      `docs/design/runtime/trace-implementation.md`.
- [ ] No diagnostic or optimisation code was added or changed, so no page under
      `docs/kcs/codes/` applies.
- [ ] No new design doc was added, so no new link in `docs/design/README.md` is
      needed; the edited docs are already linked.

## Notes for reviewers

- **History.** An early commit accidentally staged a local `tmp` symlink (the
  repo ignores `tmp/`, which does not match a symlink) and the next commit
  removed it. The pair is left in place rather than rewritten — the **net diff
  of this branch contains no `tmp` entry**, verified.
- Running the `runtime/rust` suite needs that `tmp` symlink present in the
  worktree: without it the suite silently drops about 80 tests, because the
  build script compiles libtommath from the fetched Tcl source tree.
- Three pre-existing divergences were found while testing and deliberately
  **not** widened into here: #1569, #1574, #1575.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
