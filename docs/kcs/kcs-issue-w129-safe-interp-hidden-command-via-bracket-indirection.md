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
   an explicitly scoped-out, pre-existing gap, not fixed here.** Only
   `namespace ensemble create ... -map {...}` is handled by
   `handle_namespace_ensemble` at all (`configure` is silently ignored —
   `if args[1] != "create" { return; }`); even for `create`, a `-map` target
   is recorded only as a *reference* (`push_command_reference`, for
   find-references/rename), never resolved at the ensemble's own *call
   sites* — `myEnsemble sub ...` is analysed as an ordinary call to
   `myEnsemble` (not hidden), never as a call to the mapped target. Making
   W129 see through ensemble-map redirection would require building
   generic ensemble-dispatch-aware call-site resolution that does not exist
   for *any* diagnostic today — the same theme as issue #979's
   interprocedural call-site gap. Fixing that is a distinct, larger,
   pre-existing architectural gap shared with #979, not specific to
   bracket-substitution indirection; tracked separately rather than folded
   into this fix.
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

## Known, separate gap found during this investigation (not fixed here)

`analyse_per_item` — the incremental shell/body-pass split the *live* LSP
server always uses for diagnostics (`tcl-lsp-db`) — defers **every**
proc/method body (via `DeferredBody`, filled by
`analyse_proc_body_isolated` in a second, isolated pass) regardless of
whether it is nested inside a tracked `interp eval` safe-interpreter body.
`DeferredBody` carries no `safe_interp_stack` snapshot at all, so **W129
never fires for a hidden call inside any proc (or, by the same mechanism,
`apply`-lambda) body nested in a safe interpreter when analysed
incrementally** — including a *directly-written* call with no
bracket-substitution indirection whatsoever:

```tcl
interp create -safe s
interp eval s { proc f {} { source foo }; f }
```

draws no W129 through the live server's diagnostics path, even though
`Analyser::analyse` (the whole-file walk — used by the `tcl diag`/`lint`
CLIs and every existing W129 unit test) correctly flags it. This is a
pre-existing gap, not introduced or worsened by this fix (this fix's own
`handle_apply_command` recursion inherits the exact same limitation for the
indirect case, symmetrically with the direct one) and it is **not specific
to bracket-substitution indirection** — issue #1001's actual subject — so it
is out of scope here. It is pinned by a dedicated, currently-still-red-by-
design unit test
(`safe_interp_w129_lost_across_per_item_deferred_proc_body_1001` in
`tcl-compiler`'s `analyser::handlers::tests`) so a future fix threading
`safe_interp_stack` (or an equivalent snapshot) through `DeferredBody` /
`analyse_proc_body_isolated` has a red test to turn green. Given its
real-world impact (it affects the live server's diagnostics for *any*
proc/apply body in a safe interpreter, not just this idiom), it is worth
its own tracked issue.

## Failure modes

- Checking `[list apply ...]` unconditionally (not gated on the enclosing
  argument's registry role) would make `set data [list apply {...} value]`
  — never invoked — warn, an over-broad false positive on inert data.
- Recursing into `handle_apply_command` outside a tracked safe interpreter
  would widen this walker's general scope (every other diagnostic starting
  to fire inside `[list apply ...]` bodies everywhere) — guarded against by
  gating every new call site on `!self.safe_interp_stack.is_empty()`.
- Assuming `rust/tcl-vm`'s `UNSAFE`-command list is authoritative for the
  static registry's `Traits::SAFE_INTERP_HIDDEN` set: `tcl-vm` additionally
  hides `after`/`vwait`, which the registry does not currently mark
  `SAFE_INTERP_HIDDEN` (only the 13 commands this note's companion
  diagnostic page lists are marked). This discrepancy is unrelated to
  bracket-substitution indirection and is not addressed by this fix —
  noted here so a future contributor does not assume the two sets are
  already reconciled.

## Triage checklist

1. Confirm the call site is genuinely reached through one of the three
   mechanisms above (list-quoted deferred call, direct nested substitution,
   `{*}`-expanded list-quoted head) — a bare dynamic dispatch
   (`{*}$var`/`$cmd args`) is out of scope by design (decision rule 3).
2. For a list-quoted deferred call, check the *enclosing* argument position
   actually carries `ArgRole::Body` / `LambdaLiteral` / `CommandPrefix` for
   the command it sits in (`registry.arg_indices_for_role`) — if not, the
   gate is correctly silent (decision rule 4).
3. For an `apply`-lambda-body case specifically, check whether the call is
   reached via `Analyser::analyse` (works) or `analyse_per_item` (currently
   does not, for *any* proc/apply body in a safe interpreter — the separate
   gap above, not this fix's concern).
4. If a hidden command is reached via `namespace ensemble configure -map`
   or an ensemble's call site generally, that is the pre-existing,
   documented-separate gap in decision rule 7 — not a regression in this
   fix.

## Test anchors

- `rust/tcl-compiler/src/analyser/handlers.rs` (suffixed `_1001`) —
  `safe_interp_w129_list_quoted_apply_lambda_body_reports_hidden_source_1001`
  (the reported repro), `safe_interp_w129_list_quoted_apply_in_command_prefix_position_1001`,
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
  `safe_interp_w129_lost_across_per_item_deferred_proc_body_1001` (pins the
  separate, out-of-scope per-item gap above).
- `rust/tcl-compiler/src/signature_scan/command_prefix.rs` —
  `list_quoted_command_segment` (new shared primitive); existing
  `extract_list_quoted_prefix_head` tests continue to pass against it
  unchanged.
- `rust/tcl-lsp-server/tests/e2e/issue1001.rs` — full end-to-end coverage
  against the real, packaged native server for every path that does not
  depend on the separate per-item/`DeferredBody` gap above (list-quoted
  direct hidden command in both `Body` and `CommandPrefix` positions, the
  plain-data FP guard, direct nested substitution, `{*}`-expanded
  list-quoted head, the safe-command FP guard through list-quoted `apply`).

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
