# KCS: W129 misses a hidden command reached through `[...]` bracket-substitution indirection

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors

## Symptom

**W129** (the Safe Base diagnostic — see
[its own diagnostic-code note](codes/kcs-diagnostic-w129-command-hidden-in-safe-interpreter.md)
for the user-facing picture) fires correctly when a command hidden from a
`-safe` interpreter is the *literal, written head* of a call inside that
interpreter's `interp eval` body:

```tcl
interp create -safe s
interp eval s { source b.tcl }
```

but silently misses the identical violation the moment it is reached only
through a `[...]` bracket substitution — most importantly the pervasive
`[list apply {...} $x]` deferred-command idiom:

```tcl
interp create -safe s
interp eval s {
    package ifneeded myPackage 1.0.0 [list apply {dir {
        source [file join $dir src font.tcl]
    }} $dir]
}
```

No warning appears anywhere in this snippet, even though `source` is exactly
as hidden and exactly as doomed to raise `invalid command name` at run time
as the first example. Unlike a missed style warning, a silently-missing
safety diagnostic is actively worse than not having the check at all — it
creates false confidence that the code has been vetted for safe-interpreter
compatibility when it has not.

Reported as issue #1001.

## Operational context

The entire W129 implementation is one gate,
[`safe_interp_visibility_gate`](../../rust/tcl-compiler/src/analyser/commands.rs)
(issue #945 fault 7), called from exactly one place: the top of
`process_command`. It takes a literal command-name string and a token, and
checks that name against the interpreter-domain state
(`self.safe_interp_stack`) built by the `InterpCreate` / `InterpHide` /
`InterpExpose` / `InterpEval` hooks. The *direct* case in the symptom above
works because a literal `apply {argList body} $x` head is recognised via
head-text dispatch (`AnalyserHookId::Apply` → `handle_apply_command`), which
walks into the lambda body via `analyse_body`, re-entering `process_command`
for each inner command and correctly inheriting `self.safe_interp_stack` —
so the nested `source` call hits the same gate a top-level call would.

Anything reached only through a bracket substitution never reached
`process_command` in a form the gate recognised, for three distinct
reasons, each now fixed by its own mechanism:

1. **`[list apply {...} $x]` (and any other command) sitting in one of the
   *enclosing* command's own `Body` / `LambdaLiteral` / `CommandPrefix`-role
   argument positions** (`package ifneeded`'s script argument, a `-command`
   option, `trace add ... command`, `after`/`after idle`, `eval`, `uplevel`)
   — `list`'s own arguments are never re-walked as a nested call at all; the
   main walk treats the whole `[...]` value as opaque data. Fixed by
   [`Analyser::check_deferred_call_safe_interp_hiding`]: for each such
   argument position, resolve a `[list HEAD ...]` shape via
   [`list_quoted_command_segment`] (in `tcl-compiler`'s
   `signature_scan::command_prefix`, shared with
   [`extract_list_quoted_prefix_head`] — see decision rule 1 below), gate
   `HEAD` directly, and — when `HEAD` resolves to the `Apply` hook — recurse
   into [`Analyser::handle_apply_command`] with the segment's remaining
   words as if they were a literal `apply {...} $x` call. Reusing that exact
   handler means the lambda body is walked by the same `analyse_body` →
   `process_command` recursion the direct case already uses, so a hidden
   call nested *inside* the lambda body (the reported repro's `source`) hits
   the unmodified gate with `self.safe_interp_stack` still live — no new
   logic was needed inside the gate itself, only getting the walk to reach
   the lambda body at all.
2. **A direct nested `[...]` substitution, with no `list`-quoting at all**
   (`set x [source b.tcl]`, `if {[exec ls] ne ""} ...`) — a bracket
   substitution always evaluates its content immediately, wherever it
   appears, so this is not a "deferred call" question at all; the existing
   nested-substitution walker
   (`record_nested_invocations_from_args` /
   `run_nested_command_diagnostics` → `dispatch_nested_segment`) already
   discovers every such nested command at arbitrary depth for *other*
   per-command diagnostics (arity, W100/W110/W216, ...) but never checked
   the safe-interpreter gate. Fixed by adding the same
   `safe_interp_visibility_gate` call to the top of `dispatch_nested_segment`
   — free arbitrary-depth coverage, since the discovery machinery already
   existed.
3. **`{*}[list HEAD ...]` as a command's own (expand-marked) head** — `{*}`
   expansion splices `list`'s result into the statement's own argv, so the
   command's *effective* head becomes `HEAD`, while the literal head word
   `process_command` sees is the substitution text (never a registry name).
   Fixed by [`Analyser::check_indirect_hiding`] resolving the same
   `[list HEAD ...]` shape when the head word is expand-marked, gating the
   *effective* head instead of the unresolvable literal one.

`{*}$cmdList` (an opaque variable) and a bare dynamic dispatch (`set cmd
source; $cmd $file`) are **not** flagged — see decision rule 3.

## Decision rules / contracts

1. **The list-quoted-command shape is resolved through the exact same
   primitive the highlighting/call-graph consumers use, generalised to
   return the whole segmented command, not just a head+span.**
   [`extract_list_quoted_prefix_head`] (used by `ArgRole::CommandPrefix`
   consumers: find-references, call-hierarchy, callback-arity) and W129's
   new [`list_quoted_command_segment`] both live in
   `tcl-compiler/src/signature_scan/command_prefix.rs` and share one
   resolution: the token's sole inner command must be a call to a
   `Traits::BUILDS_COMMAND_PREFIX` command (`list`) whose own first argument
   is a literal, resolvable bareword. W129 additionally needs the
   *remaining* words (to recurse into `handle_apply_command` with them),
   which the shared function now returns as the full `SegmentedCommand`;
   `extract_list_quoted_prefix_head` reduces to reading its head/span/baked
   count from that same segment. No second parsing mechanism was
   introduced.
2. **A false negative (missed real violation) is the worse failure mode for
   W129, unlike the general analyser and the highlighting consumers.** The
   [apply-lambda-highlighting KCS
   note](kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md)'s
   decision rule 4 documents that the deep SSA/CFG-based analyser
   deliberately does **not** follow `[list ...]` indirection, to stay
   conservative (a false positive there is a spurious diagnostic on inert
   data). That deep analyser is a different component from the one this fix
   touches (`tcl-compiler/src/analyser/commands.rs`'s diagnostic-producing
   walker, not the IR/CFG/SSA optimiser pipeline) — see that note's §7 for
   the explicit confirmation the SSA/CFG layer has no instances of this bug
   class and stays out of scope here. This fix's gating is still
   precision-scoped (rule 1's `deferred_role`-equivalent check — decision
   rule 4 below), it is simply on the *permissive* side of the tradeoff
   that specific check makes, appropriate for a security diagnostic rather
   than a cosmetic one.
3. **`{*}$var` and `$cmd args` (dynamic dispatch via an opaque variable) are
   deliberately left unflagged.** This matches the existing, established
   precedent elsewhere in this codebase for "prefer a miss over a false
   positive" on dynamic command dispatch (W123's unresolved-command
   handling, the command-prefix bareword-only guard in
   `command_prefix.rs`'s module docs: "A dynamic head (`$var`/`[cmd]`, in
   any shape) can't be resolved to a proc and recording it would false-fire
   W123, so it stays unrecorded"). No new precedent was invented for this
   fix; `check_indirect_hiding`'s `{*}`-head resolution only engages when
   the head word is a `Cmd`-kind (`[...]`) token, which a bare `$var` never
   is, so the guard falls out of the existing token-kind check rather than
   a new explicit branch.
4. **The new deferred-call resolution is gated on the exact `deferred_role`
   precision the highlighting consumers use — never a blanket "any `[list
   apply ...]` anywhere is a call".** `check_deferred_call_safe_interp_hiding`
   only inspects argument indices the registry marks `Body` / `LambdaLiteral`
   / `CommandPrefix` for the *enclosing* command (mirrors
   `deferred_role_arg_starts` in `tcl-lsp-core`'s `semantic_tokens.rs`, from
   the codex-review follow-up to #954) — a `[list apply {...} value]` sitting
   in ordinary `set` data is never treated as invoked, even though its lambda
   body contains a hidden command (see the FP-guard test anchors below).
5. **This entire fix is additive and gated on `!self.safe_interp_stack.is_empty()`
   at every new call site.** Outside a tracked safe interpreter — the
   overwhelming common case — none of the three new mechanisms run at all:
   no new scope is created, no new diagnostic of any kind appears, and
   `[list apply {...} $x]` used anywhere in ordinary (non-safe-interpreter)
   code stays exactly as un-analysed as it was before this fix. This was a
   deliberate choice over unconditionally following the indirection for
   *every* consumer of this analyser: doing so would have widened this
   walker's scope far beyond W129 (every other diagnostic that already
   fires inside a directly-written `apply {...}` body would newly start
   firing inside a `[list apply {...} $x]` body too, everywhere in the
   codebase, not just inside safe interpreters) — a much larger, differently
   -reviewed change than "fix W129's recall."
6. **Runtime severity finding (required investigation before treating this
   as purely an analyser-precision issue):** both `rust/tcl-vm` and the WASM
   backend (`runtime/rust`) enforce safe-interpreter command hiding at a
   single, universal command-resolution choke point
   (`InterpState::lookup_command` / the WASM runtime's equivalent
   `dispatch_inner`/namespace-resolve path) that only ever consults the
   *visible* command table — a hidden command is not merely flagged, it is
   physically absent from the table every invocation path (direct call,
   `{*}` expansion, `eval`/`uplevel` of a dynamically-built string, a
   namespace-ensemble `-map` redirect, an alias) resolves through. `rename`
   cannot resurrect a hidden command's callability either: it only
   operates on the same visible table, so it can neither find nor move a
   name that was already hidden. **This means the runtime already correctly
   rejects every indirection shape enumerated in this note at execution
   time, independent of what the static lint catches — this fix is purely
   about improving the *static diagnostic's* recall (better, earlier editor
   feedback), not closing an executable security hole.** The one adjacent,
   separately-actionable finding from that investigation: `rust/tcl-vm` has
   no test at all asserting a hidden command raises `invalid command name`
   inside a `-safe` interpreter (its only "safe child" e2e vector tests the
   opposite direction — an explicitly-wired alias still working), and
   neither `tcl-vm` nor the WASM runtime has a test exercising any
   indirection shape specifically. Worth a follow-up test-only PR; not
   folded into this fix since it changes no runtime behaviour.
7. **`namespace ensemble configure -map` redirection to a hidden command is
   now resolved at the ensemble's own call sites.** `handle_namespace_ensemble`
   now accepts both `create` and `configure` (previously `configure` was
   silently ignored — `if args[1] != "create" { return; }`); for both forms,
   `record_ensemble_map_targets` records each `-map` target's *raw, written
   text* (not a namespace-resolved qualified name) into a new
   `self.ensemble_command_maps: HashMap<String, HashMap<String, String>>`,
   keyed by the ensemble's own name (the enclosing namespace for a bare
   `create`, or the `-command` value when given — `-command` *replaces* the
   default naming, it does not add an alias, confirmed against real
   `tclsh8.6` — or the resolved `NAME` argument for `configure`). A new
   `Analyser::check_ensemble_redirect_hiding` call (from both
   `check_indirect_hiding` and `dispatch_nested_segment`) looks up the
   called command's first argument (the ensemble subcommand) in this map
   and, on a hit, runs `safe_interp_visibility_gate` against the mapped
   target exactly as if it had been called directly. Storing the *raw*
   target text (rather than resolving it through
   `resolve_command_qualified_name`) is deliberate: real Tcl resolves a
   `-map` target relative to the *ensemble's own* namespace, not the
   caller's, and a resolved qualified name would frequently be a
   synthetic, interp-domain-prefixed name (e.g. `::@interp@s::source`)
   that the registry lookup (bare names only) could never match — using
   the written text, unresolved, is a deliberate, permissive-leaning
   precision tradeoff, consistent with this fix's "prefer a false positive
   over a missed real violation" stance for a security diagnostic (decision
   rule 2). This closes the gap previously described here as scoped out;
   the remaining, distinct architectural gap from issue #979 (generic
   interprocedural call-site resolution for diagnostics other than W129) is
   untouched and still tracked separately.
8. **`rename` / `interp alias` do not defeat this gate, and needed no new
   handling.** `safe_interp_visibility_gate` checks the *literal, written*
   command name against the registry — it does not resolve through
   `self.renamed_commands` or `self.command_aliases` at all, and this is
   correct as-is: real Tcl's `rename` can only rename a command already in
   the *visible* command table (confirmed by the runtime finding in rule 6
   above), so a hidden command's name is never a valid `rename` source —
   attempting `rename source mySource` inside a safe interpreter fails at
   run time with "command doesn't exist" before this diagnostic's concern
   even arises. The standard safe-interpreter delegation pattern — the
   trusted parent bridging in a capability with `interp alias s foo {}
   source` (aliasing to the *parent's own*, non-hidden `source`) — correctly
   draws no W129 for a later `foo` call inside the child, simply because
   `foo` is never itself a `SAFE_INTERP_HIDDEN` registry name; no special
   -casing was added or needed (see the alias-bridging test anchor below).
   This is issue #1002's concern about a different bug class (registry
   lookups not resolving through renames *generally*, for other
   diagnostics); it does not apply to W129's hidden-command check.

## The live server's incremental path (`analyse_per_item`) — fixed

`analyse_per_item` — the incremental shell/body-pass split the *live* LSP
server always uses for diagnostics (`tcl-lsp-db`) — defers **every**
proc/method/`apply` body (via `DeferredBody`, filled by
`analyse_proc_body_isolated` in a second, isolated pass). `DeferredBody`
previously carried no safe-interpreter visibility information at all, so
W129 never fired for a hidden call inside any such body nested in a safe
interpreter when analysed incrementally — including a *directly-written*
call with no bracket-substitution indirection whatsoever:

```tcl
interp create -safe s
interp eval s { proc f {} { source foo }; f }
```

drew no W129 through the live server's diagnostics path, even though
`Analyser::analyse` (the whole-file walk — used by the `tcl diag`/`lint`
CLIs and every existing W129 unit test) correctly flagged it. This was a
pre-existing gap, not introduced by this fix, but with real-world impact
broad enough (it affects the live server's diagnostics for *any*
proc/apply/method body in a safe interpreter, not just this issue's
bracket-substitution idiom) that it was folded into this same change
rather than left to a separate issue.

Fixed by threading a flattened visibility snapshot through `DeferredBody`
and the `tcl-lsp-db` salsa cache: `Analyser::safe_interp_ctx_snapshot`
converts the top of `self.safe_interp_stack` (a `SafeInterpCtx` whose
`hidden_extra`/`exposed` fields are `HashSet<String>`) into a
`(bool, Vec<String>, Vec<String>)` — sorted `Vec`s, not `HashSet`s, because
this snapshot must round-trip through `tcl-lsp-db`'s
`#[salsa::interned] ItemBodyKey`, whose fields must be `Eq + Hash + Clone`,
which `HashSet` does not implement. Both `handle_proc_command`,
`handle_apply_command`, and the TclOO method-body push site in `oo.rs` now
capture this snapshot into `DeferredBody::safe_interp_ctx` at defer time;
`analyse_proc_body_isolated` pushes it back onto a fresh `Analyser`'s
`safe_interp_stack` before analysing the deferred body, so the isolated
second pass sees the same visibility state the shell pass had. `tcl-lsp-db`
carries the same field through `ItemBodyKey` end to end (both the
`ItemBodyKey::new(...)` construction site and the `DeferredBody`
reconstruction in `item_body_analysis`) so the production incremental path
gets identical coverage to `Analyser::analyse`, not just the test-only
`analyse_per_item` entry point.

One narrower limitation remains, deliberately not fixed, because it would
require threading interpreter *identity* (`interp_path_stack` /
`self.interpreters`), not just a visibility snapshot, through
`DeferredBody` — a larger, separate change: a `proc` that locally
*redefines* a hidden name **nested inside** a deferred body does not
suppress W129 for a later call to that redefined name within the *same*
body, under incremental analysis only (`Analyser::analyse`'s whole-file
walk is unaffected, since it never loses interpreter identity in the first
place). Concretely:

```tcl
interp create -safe s
interp eval s {
    proc f {} {
        proc source {args} { return ok }
        source foo
    }
    f
}
```

still draws W129 on the inner `source foo` call under `analyse_per_item`,
even though the local redefinition makes it a false positive (the call
truly is safe at run time). This can only ever produce a spurious
diagnostic, never miss a real violation, so it stays on the acceptable
side of this fix's "prefer a false positive over a missed real violation"
stance (decision rule 2) — unlike the gap this whole fix closes, which
could miss a real violation. Pinned by
`safe_interp_w129_nested_redefinition_inside_deferred_body_still_flagged_1001`,
which documents and asserts the current (over-flagging) behaviour rather
than silently regressing it in either direction.

## Failure modes

- Checking `[list apply ...]` unconditionally (not gated on the enclosing
  argument's registry role) would make `set data [list apply {...} value]`
  — never invoked — warn, an over-broad false positive on inert data.
- Recursing into `handle_apply_command` outside a tracked safe interpreter
  would widen this walker's general scope (every other diagnostic starting
  to fire inside `[list apply ...]` bodies everywhere) — guarded against by
  gating every new call site on `!self.safe_interp_stack.is_empty()`.
- `rust/tcl-vm`'s `UNSAFE`-command list previously over-hid `after` and
  `vwait`, disagreeing with the static registry's `Traits::SAFE_INTERP_HIDDEN`
  set (only the 13 commands this note's companion diagnostic page lists are
  marked, and real `tclsh8.6` confirms neither `after` nor `vwait` is hidden
  by `-safe`). Fixed by removing both from `UNSAFE` in both
  `rust/tcl-vm/src/interp.rs` and the WASM runtime's `runtime/rust/src/interp.rs`
  equivalent, with a pinning comment; verified with
  `after_and_vwait_remain_callable_in_a_safe_interp` (see test anchors).
  This discrepancy was unrelated to bracket-substitution indirection but,
  like the per-item gap above, was folded into this same change given its
  direct relevance to this issue's runtime-severity investigation
  (decision rule 6).

## Triage checklist

1. Confirm the call site is genuinely reached through one of the three
   mechanisms above (list-quoted deferred call, direct nested substitution,
   `{*}`-expanded list-quoted head) — a bare dynamic dispatch
   (`{*}$var`/`$cmd args`) is out of scope by design (decision rule 3).
2. For a list-quoted deferred call, check the *enclosing* argument position
   actually carries `ArgRole::Body` / `LambdaLiteral` / `CommandPrefix` for
   the command it sits in (`registry.arg_indices_for_role`) — if not, the
   gate is correctly silent (decision rule 4).
3. For an `apply`/proc/method-body case, both `Analyser::analyse` (the
   whole-file walk) and `analyse_per_item` (the live server's incremental
   path) now carry safe-interpreter visibility into deferred bodies; if
   only one of the two flags a case, that is a real regression, not the
   old known gap (which is fixed — see the section above).
4. If a hidden command is reached via `namespace ensemble create` or
   `configure -map`, `check_ensemble_redirect_hiding` should catch it
   (decision rule 7) — if it doesn't, check whether the `-map` target text
   or the ensemble key (bare vs. explicit `-command`) actually matches what
   `record_ensemble_map_targets` stored, before assuming a new gap.
5. A local redefinition of a hidden command's name **nested inside** a
   deferred body (proc/apply/method) not suppressing a later same-body call
   under `analyse_per_item` is the one remaining, deliberately-accepted,
   over-flagging-only limitation — see the end of the per-item section
   above, not a bug to chase.

## Test anchors

- `rust/tcl-compiler/src/analyser/handlers.rs` (suffixed `_1001`) —
  `safe_interp_w129_list_quoted_apply_lambda_body_reports_hidden_source_1001`
  (the reported repro), `safe_interp_w129_expand_list_quoted_apply_lambda_body_1001`
  and `safe_interp_w129_list_quoted_apply_package_ifneeded_then_require_1001`
  (issue #1001's own second and third repro cases, pinned verbatim),
  `safe_interp_w129_list_quoted_apply_in_command_prefix_position_1001`,
  `safe_interp_w129_list_quoted_apply_after_idle_1001`,
  `safe_interp_w129_list_quoted_hidden_command_directly_1001` (no `apply`),
  `safe_interp_w129_list_quoted_apply_in_plain_data_is_not_flagged_1001` (FP
  guard, mirrors #954's non-invocation guard),
  `list_quoted_apply_lambda_outside_any_safe_interp_is_untouched_1001` (no
  scope-creep guard), `safe_interp_w129_direct_nested_bracket_substitution_1001`,
  `safe_interp_w129_direct_nested_bracket_substitution_deep_1001`,
  `safe_interp_w129_expand_list_quoted_head_1001`,
  `safe_interp_w129_expand_dynamic_var_head_not_flagged_1001` (TN),
  `safe_interp_w129_dynamic_variable_dispatch_not_flagged_1001` (TN),
  `safe_interp_w129_eval_list_quoted_hidden_command_1001`,
  `safe_interp_w129_uplevel_list_quoted_hidden_command_1001`,
  `safe_interp_w129_list_quoted_apply_safe_command_not_flagged_1001` (TN),
  `safe_interp_w129_redefined_command_not_flagged_through_indirection_1001`,
  `safe_interp_w129_alias_bridged_command_not_flagged_through_indirection_1001`,
  `safe_interp_w129_ensemble_create_map_redirect_to_hidden_command_1001`,
  `safe_interp_w129_ensemble_configure_map_redirect_to_hidden_command_1001`,
  `safe_interp_w129_ensemble_default_name_map_redirect_to_hidden_command_1001`,
  `safe_interp_w129_ensemble_map_redirect_to_safe_command_not_flagged_1001`
  (TN), `ensemble_map_redirect_outside_any_safe_interp_is_untouched_1001`
  (no scope-creep guard),
  `safe_interp_w129_reaches_deferred_proc_body_under_per_item_1001`,
  `safe_interp_w129_reaches_deferred_apply_body_under_per_item_1001`,
  `safe_interp_w129_reaches_deferred_method_body_under_per_item_1001`,
  `deferred_proc_body_outside_any_safe_interp_is_untouched_under_per_item_1001`
  (no scope-creep guard),
  `safe_interp_w129_nested_redefinition_inside_deferred_body_still_flagged_1001`
  (pins the narrower, accepted, over-flagging-only limitation described
  above).
- `rust/tcl-compiler/src/signature_scan/command_prefix.rs` —
  `list_quoted_command_segment` (new shared primitive); existing
  `extract_list_quoted_prefix_head` tests continue to pass against it
  unchanged.
- `rust/tcl-lsp-server/tests/e2e/issue1001.rs` — full end-to-end coverage
  against the real, packaged native server for every path that does not
  depend on the (now-fixed, but deliberately not re-exercised at the e2e
  layer) per-item/`DeferredBody` case (list-quoted direct hidden command in
  both `Body` and `CommandPrefix` positions, the plain-data FP guard, direct
  nested substitution, `{*}`-expanded list-quoted head, the safe-command FP
  guard through list-quoted `apply`).
- `rust/tcl-vm/tests/safe_interp_e2e.rs` (new) — runtime-severity coverage
  confirming `tcl-vm`'s bytecode VM already enforces hidden-command
  rejection at execution time, independent of the static lint, for every
  indirection shape this note enumerates:
  `hidden_command_direct_call_raises_invalid_command_name`,
  `hidden_command_via_expand_list_quoting_raises_invalid_command_name`,
  `hidden_command_via_eval_of_built_string_raises_invalid_command_name`,
  `hidden_command_via_ensemble_map_redirect_raises_invalid_command_name`,
  `rename_cannot_resurrect_a_hidden_command`,
  `after_and_vwait_remain_callable_in_a_safe_interp` (pins the
  `UNSAFE`-list fix above),
  `safe_interp_hides_every_implemented_command_in_the_registry_set`.

## Related

- [KCS index](README.md)
- [W129 diagnostic-code note](codes/kcs-diagnostic-w129-command-hidden-in-safe-interpreter.md)
- [apply lambda body not highlighted via list quoting](kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md)
  — the prior fix (#954/#999) that built the `Traits::BUILDS_COMMAND_PREFIX`
  / `ArgRole::LambdaLiteral` / `deferred_role` infrastructure this fix reuses
- [Glossary](../GLOSSARY.md)

[`safe_interp_visibility_gate`]: ../../rust/tcl-compiler/src/analyser/commands.rs
[`extract_list_quoted_prefix_head`]: ../../rust/tcl-compiler/src/signature_scan/command_prefix.rs
[`list_quoted_command_segment`]: ../../rust/tcl-compiler/src/signature_scan/command_prefix.rs
[`Analyser::check_deferred_call_safe_interp_hiding`]: ../../rust/tcl-compiler/src/analyser/commands.rs
[`Analyser::check_indirect_hiding`]: ../../rust/tcl-compiler/src/analyser/commands.rs
[`Analyser::handle_apply_command`]: ../../rust/tcl-compiler/src/analyser/handlers.rs
