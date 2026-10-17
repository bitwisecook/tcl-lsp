# Luna-7 delta review: Sol-5 `same-document-order-v3.patch` (V2 → V3)

**Verdict: HOLD.** One narrow item remains, with a mechanical fix.

Every exit path I raised against v2 is now handled. But four more shapes in the same class still let the document end revoke a removal after the load completed normally. All four are confirmed in tclsh 8.6 and 9.0:

- `apply` with a lambda held in a variable;
- `lsort -command`;
- `expr` on an unbraced variable;
- `regsub -command`, in Tcl 9.0 only.

The fix is to widen the end-only check from the `Body` role to every role that can run a script. That uses registry data only, and test-400, test-404 and test-406 still flip.

## Review basis

- **Patch:** sha256 `00cfac7a7d11a53887311f3ef34492d322c3b53b13bb6a801814217f6257d719`.
- **Baseline:** the `a/` copies match HEAD `1433f2ffe`, and `git apply --check` passes.
- **Delta reviewed:** `sol-5/v2/b` against `sol-5/b`.
  - `source_graph.rs` (the end-only cutoff, the deferred marking, the capture split and two controls), the `workspace_index.rs` doc comments, §8.1, and the proof scope changed.
  - `namespace_import.rs`, `definition.rs` and `shared-utility-contracts-rust.md` are unchanged from v2.
- **Probes:** in `scratchpad/agents/luna-7/probe/v3/`. Each one loads, in that order, an import, the exit shape, a forget, and a flag recording that the end was reached.
- **Method:** static review only. Nothing was compiled or run.

## The document-end definition and the end-only cutoff

These parts are sound:

- The document end now means "after the document completes normally". An error path never reaches it.
- `contains` (`b/rust/tcl-lsp-core/src/source_graph.rs`) admits a point at or past the end only when there is no positional cutoff and `end_withheld` is false.
- `record_exits` (`b/source_graph.rs:1120-1142`) sets `end_withheld` for every command outside definition bodies and stored callbacks when any of these hold:
  - its traits include `TRANSFERS_CONTROL`;
  - it selects `BodyExecutionSpec::SourceFile`;
  - it runs an unretained `Body` script now;
  - its words are unresolved, unvalidated or not `Selected`;
  - it has no schema.
- Positioned queries are untouched, which is correct: they are reach-conditioned.

### The exit paths from my v2 review: all closed

| path | how v3 handles it |
|---|---|
| sourced file doing `return -code return` or `return -level 2` (x_src, x_lvl) | `source` selects `SourceFile`, so the end is withheld |
| user proc doing `return -code return` (x_proc) | its words are not `Selected`, so the end is withheld |
| `break`, `continue`, `tailcall`, `yield` (x_brk) | `TRANSFERS_CONTROL`, so the end is withheld |
| `return` inside a `namespace eval` body whose words don't resolve (x_nsret) | unresolved words, so the end is withheld |
| failed export or import (qual.tcl, conflict.tcl) | an error path never reaches the end under the new definition |
| dynamic heads, `{*}$c` (x_expand) | unresolved words |
| `eval $s`, `uplevel $s`, `interp eval {} $s`, `namespace inscope :: $s` | `Body` role that runs now but is unretained |

### New gap: shapes that still revoke at the end

`may_run_unretained_script` (`b/source_graph.rs:920-947`) checks only `ArgRole::Body`. A `Selected` built-in that runs a script now through another role therefore sets nothing. tclsh 8.6 and 9.0 results (`probe/v3/run.tcl`):

| shape | source rc | after load | v3 at the document end |
|---|---|---|---|
| `set l {{} {return -code return}}; apply $l` (role `LambdaLiteral`, flow `Lambda`) | 0 | alias alive, end not reached | revoked |
| `proc cmp {a b} {return -code return}; lsort -command cmp {b a}` (`CommandPrefix` run now; `cmp`'s body is a definition body and is skipped) | 0 | alive | revoked |
| `lsort -command [list apply {{a b} {return -code return}}] {b a}` | 0 | alive | revoked |
| `set e {[return -code return]}; expr $e` (role `Expr`, flow `ConcatenatedExpression`) | 0 | alive | revoked |
| `regsub -all -command a abc r`, Tcl 9.0 only (the replacement word is roled `FormatString`) | 0 on 9.0 | alive | revoked |

These shapes behave correctly and need no change:

- `subst $t` absorbs `return`; the load continues and the forget runs.
- A `trace` callback and `package require` with an ifneeded script that returns turn the code into an error, so the end is never reached.
- `apply` with a literal lambda is cut by the `return` inside its retained body.

**Fix (end-only, registry data):** in the `end_withheld` check, treat a command whose flow is not `Deferred` as a possible early completion when:

- any operand with a script-carrying role (`carries_script()`, `OpaqueScript` or `CommandPrefix`) is not retained as a region; or
- any operand has role `CommandPrefix`, because the invoked command is not visible as a command.

Add controls for `apply $l`, `lsort -command cmp` and `expr $e`.

`regsub -command` is a registry data gap. The replacement word should be resolved as `CommandPrefix` under `-command`. Fix it in the regsub role resolver, or list it in the callback-surface inventory follow-up.

Tests 400, 404 and 406 use none of these shapes, so they still flip.

### Positioned queries (unchanged): sound

The new deferred marking only stops commands inside stored callbacks from setting exits:

- Synchronous trace callbacks turn non-OK return codes into errors (`probe/v3` x_trace).
- `package ifneeded` scripts do the same through `package require` (x_pkg).
- Event-loop callbacks cannot unwind the script being sourced.

So skipping them is sound.

## New control: `document_end_is_after_normal_completion_only`

The control is correct for its three shapes (`source`, a user proc returning, `break`):

- The positions before and after the exit stay `KNOWN`.
- The document end is not `KNOWN`.

Extend it with the four gap shapes above once they are fixed.

## The `rename myeval eval` control and the Realm barrier

This is reasonable reliance:

- `original_source_registry_barrier` refuses registry source at an offset when the source-transition tape's command cell for the head is `Alias`, `Shadowed`, `Deleted`, `SourceProcedure` and similar (`rust/tcl-compiler/src/command_binding/source_transition_advice.rs:1248, 1751-1770`).
- After `rename myeval eval`, the `eval` cell should therefore be a source procedure, so the words are not `Selected` and the body is not entered.

Two caveats:

- The barrier returns `false` when the realm has no source lexer config or image (`realm/source_transition_advice.rs:278-302`). In that case the control fails, which is fail-loud.
- The control executes the proof; until it runs, the reliance is unverified.

§8.1 "Known limits" documents the cross-document and `interp alias` residual accurately.

## Low items: all resolved

- The `link_alias_live` doc is no longer self-contradictory (`b/workspace_index.rs:8735-8750`).
- The stale citation now names `exported_original_slot_at_import_in` (`:8794`).
- `DocumentSequence::capture` is about 67 lines, under the clippy limit (`b/source_graph.rs:979-1056`). The new helpers are small. No `#[allow]` was added and no command name is matched.
- The proof scope now includes the document-end definition.

## Still open, unchanged from v2

- The proof gate, `xtask owner-resolution` (three rows) and clippy were not run.
- Timing is still needed: `settling_invocations_does_not_scale_with_imports_times_exports` (< 2 s), test-172 and large-corpus references. Capture now also resolves bodies for `Deferred`-flow commands.
- The second and third 1b45c540e controls need sign-off.
