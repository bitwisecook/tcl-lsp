# Sol-13: namespace-statement residual scope (Task 1) and the `set after 6` gap (Task 2)

- **Baseline:** HEAD d0cd9d73b. All line numbers below are `git show HEAD:` lines. Extracted copies are in this directory: `cb.rs`, `oct.rs`, `ci.rs`, `sta.rs`, `be.rs`, `ncs.rs`, `nc.rs`, `ns.rs`, `source_command_world.rs`, `original_namespace_binding.rs`, `sg.rs`.
- **Method:** static reading only, plus `nice tclsh8.6/9.0` probes in `probe/` (`results.txt`). Nothing was run in /home/user/tcl-lsp. There is no patch: see "Why no patch" at the end.

## Task 2: why `set after 6` gets no words (cause established statically)

Fixture (`source_graph.rs` 3350):

```
set before 1
namespace eval ::n {
    set inner 2
}
if {$x} {set branch 3}
proc q {} {set body 4; return}
set after 6
```

### Causal chain

1. **The condition read always fails.** `if {$x}` evaluates its braced condition through:
   - `walk_expression` → `ExprNode::Var` (cb.rs 11941-11990);
   - `walk_expression_variable` (executed_expression_source.rs 237);
   - `observe_variable_substitution` (cb.rs 9720-9820);
   - `source_representation::unobserved_read_result` (source_representation.rs 203-218).

   `x` is never set in the document. The authoring entry is closed, and the cell is unobserved and static. So `VariableContext::read_is_definitely_missing` (var_resolve.rs 2686-2711: contents `Undefined`, no observers) is true. The read therefore returns `SourceOutcomes::invocation(.., Tcl(Error))`: an outcome with **no normal state**. The `if` has no normal continuation.

   This is correct Tcl for a fresh interpreter: `can't read "x": no such variable`.
2. **The walk stops having a normal state.** In `walk_source_segments` (cb.rs 8599-8611), `outcomes.normal` is now `None`. The walk stops dispatching and calls `record_unentered_declaration_suffix` (declaration_layout.rs 2387-2445) for the rest of the script.
3. **The suffix recorder stops at `proc q`.** That recorder only records *declaration operand layouts*: no dispatch points, no arguments, no compiler visits. It `break`s at the first command whose `ScriptBodyFlow` is not `None`. `proc q {...}` has a deferred body, so the recorder stops there.
4. **`set after 6` gets nothing.** It has no `SourceBindingPoint` and no layout. `invocation_at_source` falls into `query_dispatch_points` with no points, which yields `SourceInvocationBinding { unknown: true, .. }` (cb.rs 8307-8318), with no head input and no lookup state. Every issuer in `source_registry_words` then returns `None`.
   - It is not the Realm/tape barrier.
   - It is not the body_effects.rs 1534-1542 fork/join barrier. That code only runs for commands inside a retained `source_body`, and `set after` is at the root.

### Bisection (static predictions for Luna-12's four variants)

| fixture | `set after` words | END |
|---|---|---|
| `if {$x}` alone, then `set after 6` | declaration-layout words only, because the suffix recorder reaches `set` (flow None). Not `Selected`. | withheld |
| `proc q` alone | `Selected` | KNOWN |
| `set x 1` before `if {$x}` | `Selected`. The read is defined, so the walk continues normally. | KNOWN |
| `if {[gets stdin]}` | `Selected`. No definitely-missing read. | KNOWN (the condition makes no math-function call) |
| original (both) | none at all | withheld |

So both parts are needed for "no words at all":
- **`if {$x}`** puts the rest of the script on an error-only path.
- **`proc q`** stops the layout-only suffix recorder before it reaches `set after`.

The variable read is the cause. The analyser fork/join barrier is not involved, and the `proc` definition matters only as the suffix stopper.

### Does it affect every file with `if {$var}`?

**No. It affects every file with a definitely-missing read.** That means a read the compiler proves undefined, made by any command at a sequence position (root or an entered body): `if {$v}`, `puts $v`, `set y $v`, `expr {$v}`. The read counts when all of these hold:
- the variable is never set earlier in the document;
- it is not provided by the authoring entry;
- it has no traces;
- it is not dynamic;
- it is not a procedure-frame cell (`SelectedFrame` owners are excluded).

After such a read:
- every later command has no dispatch point;
- later commands get only declaration-layout words up to the first body-carrying or non-literal command;
- commands after that get no words at all;
- the END is withheld.

Files that set the variable first, or read it only inside procedure bodies, are unaffected.

This is consistent with the model, because under a fresh interpreter the document never completes normally. But it is a model-scope issue for LSP documents that are sourced by a file defining the global (`if {$debug}`). The closed authoring entry then makes the rest of the file dead. I am flagging it, not fixing it: it is out of scope for the order work.

### Recommendation for the guard test

The original assertion `purpose("set after", "END") == KNOWN` is vacuously true in relation terms: `set after` is never reached. But the implementation cannot place an unreached command. Keep Luna-12's intent ("a non-exiting conditional branch before the end does not withhold it") and make the read defined but unknown. For example, insert `set x [file exists y.tcl]` before the `if`:
- it is a stock builtin;
- its value is unknown;
- it runs no callbacks.

Then restore `assert_eq!(purpose("set after", "END"), KNOWN)` on that fixture. As a separate control, add the unset-`x` fixture asserting END is withheld, with a comment that the document never completes normally (tclsh: `can't read "x"`).

## Task 1: why an unmodelled namespace statement in a body unselects every later command

### Mechanism: how the poison becomes global (traced)

- **Where the poison comes from.** All later commands lose `Selected` because `ModuleCommandBindings::opaque_domain` becomes true. That gives:
  - `UnknownCurrentLookup` (original_command_table.rs 2251-2257);
  - via `original_command_slot_paths_for_bytes` → `None` (oct.rs 1955), `UnknownLookupCoordinates` and `UnretainedHolder`.

  This is exactly Sol-5's observed obligation set for root `namespace eval` #2.
- **Which setter does it.** The only setter of `opaque_domain` on this path is `mark_opaque_binding_mutation` (cb.rs 14449-14460).
  - The sibling `original_command_world.withdraw()` on a failed `world.import`/`forget` (source_command_world.rs 1971-1979) gives unknown coordinates, but it alone does not give `UnknownCurrentLookup`.
  - The namespace-transition handlers are not the source. `apply_namespace_transition` (cb.rs 19340-19440) already scopes every unknown pattern: `unknown_lookup_namespaces` for import/forget, `unknown_export_namespaces` for export, `unknown_namespace_paths` for path. So the transition semantics are already scoped. The global poison comes from the **invocation-execution residual** around them.
- **Which residual site fires.** There are three candidate residual sites. Sol-5 observed:
  - the import is `compiled_execution_unknown`;
  - the import's own Conditional words carry **no** `UnknownCurrentLookup`.

  So the pre-dispatch site cb.rs 8816 (`compiled.unknown && !compile_error && !compiler_preserves_names() → mark_opaque_binding_mutation`) did **not** fire. Had it fired, the import's own point (recorded after 8816) would carry `UnknownCurrentLookup`. The poison is therefore applied *after* the point, in the dispatch residual:
  - **cb.rs 9234-9312, `walk_unknown_compiler_handler`.** With `compiled.unknown`, the bounded path needs two things:
    - `compiled.stable_handler_after_original_arguments(head, incoming, ns)`;
    - `compiled_invocation::successful_handler_transfer(..)`, ci.rs 2667-2760.

    Otherwise it takes the `else` branch (9307-9311): `residual.mark_opaque_binding_mutation()`. **Most likely culprit.**
  - **cb.rs 9318-9345, `walk_named_source_command`.** It marks the residual when the late lookup `with_late_lookup` (named_invocation.rs) is unknown (9341-9344), or when `ProjectedNamedArguments::capture` fails (9335-9338).
- **What is body-specific.** Two discriminating predicates differ between a root `namespace export h` (which Sol-5 says keeps `Selected`) and the same command in `namespace eval ::src {..}`. Either or both:
  - **(a) Compiled selection.** The selection for the `NamedEnsembleInvocation` grammar (namespace_.rs 927-1006: export, forget, import, path and unknown map to `::tcl::namespace::<member>`) goes through ci.rs 1002-1110. Namespace-relative inputs there are:
    - the head lookup `compiler_targets_for_original_input(head, ::src)`;
    - `before`;
    - `stable_compiler_handler`.

    `select_for_facts` returns `Unknown` for every named compiler unless `original_registered_compiler_preparation` succeeded (nc.rs 3307-3309). Any `None` in the original-word projection (ncs.rs 75-130), or a dependency not closed in `compiler_dependencies` (ci.rs 1731-1763), makes `compiled.unknown` true.
  - **(b) Handler transfer.** `successful_handler_transfer` resolves `NormalHandlerImplementationLookup::Required(lookup)` with the **legacy label lookup** `source_binding(state, "namespace", &variable_frame_namespace_key())` (ci.rs 2709-2731). In a body this looks up from `::src`, not the root. It then needs `proved_handler_target()`, which needs `source_lookup_is_closed` (command_presence.rs 397-410) for an allocated `::src` key. If that is not closed, the transfer is `None` and the code reaches 9309.

    At the root, `head` resolves from `::`. This is the only namespace-relative step in the transfer. I judge it the prime suspect, but I could not prove it statically.
- **A discriminator already exists in the compiler tests.** `command_binding::tests::source_c_import_follows_the_original_command_object` (cb.rs 21344) uses the *same* shape:
  - `namespace eval a {proc f ..; namespace export f}`;
  - `namespace eval b {namespace import ::a::f}`;
  - then it asserts a **proved** target at the final `::b::f`.

  A proved target is impossible under `opaque_domain` (`original_targets_for_input` → None → unknown). The test uses `source_proof`: `build_default()`, `authoring_native_compilation()`, 8.6, no metadata context, no declared contracts. The LSP realm instead comes from `realm::document_realm_bindings_with_resolved_input` (realm.rs 820-845), which differs in three ways:
  - `metadata_context: SuppliedSource(input)`;
  - the environment's `context.commands()`;
  - `declared` contracts (the document's own `p`/`f`).

  So:
  - **If that compiler test passes at HEAD**, the poison is caused by one of these three option differences, which is a two-run bisect.
  - **If it fails**, the poison is intrinsic to bodies, and (a)/(b) above apply.

### Decisive trace (for the orchestrator: one run, no code change)

Run `definition::tests::a_forget_after_the_import_stops_resolving_the_alias` (debug build) with:

```
TCL_LSP_NATIVE_CODEGEN_DIAGNOSTIC=1   # unconditional eprintln in tcl-compiler (ci.rs 412-470, 1192-1200)
TCL_LSP_TRACE_ORIGINAL_COMMAND_TABLE=1 # cfg(debug_assertions) eprintln in original_namespace_binding.rs 343-406, 548-556, 588-604
```

Read the output as follows:
- **`NATIVE_ORIGINAL_COMPILER_TARGET head=b"namespace" namespace=Some("::src") … unknown=`.** This tells (a) apart from (b).
- **`ORIGINAL_NAMESPACE_BINDING_CAPTURE site=52 quiet=… current=… context=…`.** This shows whether the precise seal (`OriginalNamespaceBinding::capture`, original_namespace_binding.rs 484-574) is refused, and on which premise. A captured seal turns the route into `Tcl(Ok)` and closes the handler (cb.rs 10191-10205, 10240-10245). This is the "model precisely" option that already exists.
- **`ORIGINAL_NAMESPACE_BINDING_COMPLETED … complete=`.** This lists each field that fails.

The first `opaque=true` belongs to the first command after the culprit.

### Design: the compiler-side fix

Keep `Selected` as the LSP rule. No LSP change is needed beyond fix 2 of v6, which stays gated on this. There are two layers, and both are needed.

#### Layer A: close the existing bounded paths in bodies (root cause)

Make whichever premise the trace shows failing namespace-independent:
- **If it is (b):** in `successful_handler_transfer`, resolve the head through the original slot owner, `source_binding_from_original_input(state, head_input, &frame_ns)`, which `before` already uses, instead of the legacy label `source_binding(state, head, &namespace)`. The proof is the same (a single registry-backed builtin target, with mapping plus `implementation_dependency` closed). It must not use a label route that cannot close for allocated namespace keys.
- **If it is (a):** fix the specific `None` in the named-ensemble preparation chain for nested images, or in the dependency chain.
- **If `OriginalNamespaceBinding::capture` refuses:** fix the refused premise, because that seal *is* the precise model of export, import and forget. It already requires all of the following, which are exactly the soundness premises below:
  - `quiet` (no command observers, no opaque domain, closed entry);
  - a current registry builtin handler;
  - no callbacks;
  - literal original operands;
  - the auto-import preload slot Missing (`import_is_fresh_and_closed`, original_namespace_binding.rs 664-672).

With layer A, the 404, 022, 024, 028 and 029 shapes become fully modelled, so `opaque_domain` is never set.

#### Layer B: scope the residual that stays unmodelled (Luna-12's recommendation, as Registry data)

When layer A still cannot model a namespace statement (for example, a non-literal pattern or an unknown target namespace), the dispatch residual at cb.rs 9307-9311 and 9335-9344 should be scoped when the following premises hold, instead of being the global `mark_opaque_binding_mutation()`.

**Registry data, with no command names in consumers.** Add one field to `StateTransitionDescriptor` (state_transition.rs 1814), for example:

```rust
/// What a correctly dispatched stock handler can still affect when its
/// operands are not modelled. Global keeps today's behaviour.
pub residual_scope: TransitionResidualScope,
pub enum TransitionResidualScope { Global, TargetNamespace }
```

Set `TargetNamespace` on `NAMESPACE_EXPORT_TRANSITIONS`, `NAMESPACE_IMPORT_TRANSITIONS`, `NAMESPACE_FORGET_TRANSITIONS` and `NAMESPACE_PATH_TRANSITIONS` (namespace_.rs 596-700). Every other descriptor defaults to `Global`. Declare the field on the SpecTcl/native parity surface as AGENTS.md requires, or add a `GAPS` entry. The consumer maps each declared `NamespaceTransition` to the scoped set that `apply_namespace_transition` already uses for unknown patterns:

| transition | scoped residual |
|---|---|
| `Export` | `unknown_export_namespaces ∋ target`. This rebinds no head. A later import from it already widens its own destination (`apply_import`, original_namespace_binding.rs 184-188). |
| `Import`, `Forget` | `unknown_lookup_namespaces ∋ target`. `original_lookup_key_for_slot` (oct.rs 2533-2537) and `original_command_slot_paths_for_bytes` (oct.rs 1990) then refuse every relative head in the target and every key held by it, from any namespace, path, alias or later import. |
| `SetPath` | `unknown_namespace_paths ∋ target`. This is consumed at oct.rs 2008. |

**Admission premises.** All must hold, otherwise the residual stays Global:
1. `before` (the head lookup at the site) proves exactly one target that is:
   - registry-backed `Builtin`;
   - generation 0;
   - without a prefix;
   - `retained_target_is_current`.

   This refuses n1, where the later `namespace` head in `::a` is the imported proc.
2. The named private worker `lookup.slot` (`::tcl::namespace::<member>`) is current in the *incoming* state (`implementation_dependency`, ci.rs 2158-2183, plus `original_ensemble_mapping_holds`). This refuses a renamed or redefined worker.
3. `quiet(incoming)`, as defined in original_namespace_binding.rs 339-346: no opaque domain, closed entry, no step or execution observers, and `command_observers.is_quiet()`. This refuses n2 and n3 (already opaque) and the new n9 and n10 (delete traces).
4. For `Import`, the registry's `NamespaceTransition::import_preload_command(policy)` slot is Missing in the original table, as `import_is_fresh_and_closed` checks. This refuses the new n8 (auto_import hijack).
5. `facts.effects.callback().kinds == NONE`, and the target namespace resolves (`namespace_target_key_at` is Some).

**Effect on Selected words.**
- Root heads keep `Selected`, so root `namespace eval ::a {..}` enters its body.
- Relative heads inside the target namespace lose `Selected`, so their pattern words and the END stay withheld.

### Soundness against probes (tclsh 8.6 and 9.0 agree; `luna-12/probe/results.txt` and `probe/results.txt`)

| probe | runtime | layer B answer |
|---|---|---|
| n1: import brings a renamed `namespace` into `::a` | the forget is a no-op, and `::a::h` is alive | The import into `::a` is either modelled precisely (then `::a::namespace` is an import and not Selected) or scoped (`::a` is in `unknown_lookup_namespaces`, so not Selected). The forget's words are refused, and so is the END. **Refused.** |
| n2: `proc $n` for `eval` | `eval` not run | `mark_dynamic_proc_binding` is global (cb.rs 14464-14474) and untouched. **Refused.** |
| n3: `proc $n` for `incr` (`return -code return`) | early completion | as n2. **Refused.** |
| n4: `namespace path ::p` with `::p::set` | early completion | Precise path: `set` resolves to `::p::set` (a proc). Unknown path: `::a` is in `unknown_namespace_paths`. **Refused.** |
| n5: `namespace eval :: {namespace import -force ::q::eval}` | forget not run | The target is the root, so the root is in `unknown_lookup_namespaces` and every relative root head is unknown. **Refused.** |
| n6: import of a renamed `set` into `::a` | early completion | `set` in `::a` is refused. **Refused.** |
| **n8 (new): user `proc auto_import` in a body import** | the root `incr` is rebound, and the document completes early (after=0) | premise 4 fails → Global. **Refused.** |
| **n9 (new): delete trace on `::a::h`, then a body forget** | the trace rebinds the root `incr` (after=0) | premise 3 fails → Global. **Refused.** |
| **n10 (new): delete trace, then `import -force`** | the same (after=0) | premise 3 fails → Global. **Refused.** |
| a1: the 404 shape | the root `incr` and `set` run, after=1, and `::dst::p` is gone | Admitted. |
| a2: `::m` exports a proc named `set` with no import | the root `set` and the `::a` `set` are stock (after=1) | The export cannot rebind a head. Admitted. |

**n8 to n10 are new.** They refute the task's premise that `import`/`forget` "can only rebind commands in the current namespace". They can rebind anything through the auto_import preload and through command delete traces. So a scope declared only in the Registry is insufficient without premises 3 and 4.

### Compiler tests to add with the fix

Add these in `rust/tcl-compiler/src/command_binding.rs` tests. Use a helper that mirrors `document_realm_bindings_with_resolved_input` (metadata context and declared contracts) as well as `source_proof`.

**Admitted.** For the 404 source, then `set x 0\nincr x\nset after 1`, the following must hold:
- the binding at root `namespace eval dst` #2;
- the binding at `incr x`;
- the binding at `set after`.

For each: `!execution_is_unknown()`, `proved_target()` is the stock command, and `!state.has_opaque_domain()`. Also: the export and import in their bodies have no `UnknownCurrentLookup` in `original_catalogue_source_candidate`.

**Scoped (layer B).** For `namespace eval ::a {namespace import $p}` (a non-literal pattern), then `set after 1`:
- the root `set` is proved;
- in a later `namespace eval ::a {set y 1}`, the `set` is not proved;
- `namespace eval ::a {::set y 1}` is proved.

**Refusals.** These are the n1 to n6 and n8 to n10 sources from `luna-12/probe` and `probe/`. Each names a checked command, and its binding must have no `proved_target()`:

| probe | checked command |
|---|---|
| n1 | the `namespace` head in `::a` body #2 |
| n2 | `eval` |
| n3 | `incr x` |
| n4 | `set` in `::a` |
| n5 | `eval` |
| n6 | `set` in `::a` |
| n8 | `incr x` |
| n9 | `incr x` |
| n10 | `incr x` |

In tcl-lsp-core, add the same sources to `document_sequence_refuses_shadowed_or_namespace_local_entering_commands` or `document_end_is_after_normal_completion_only`.

## Why no patch

Layer A's edit point depends on which of three body-specific premises fails: compiled selection, the handler transfer's label lookup, or the namespace-binding seal. I narrowed it, and the trace above settles it in one run. But I could not determine it statically with confidence, and a guessed edit risks a wrong fix. Layer B on its own does not fix the 404 shapes if the residual is never reached in a scoped form. It also needs the same premise proofs (2 to 4) that layer A exposes. Both layers are fully specified above. The probe sources and results are in `probe/` (`run.tcl`, `n8`–`n10`, `a1`, `a2`, `results.txt`).

## Addendum: orchestrator run evidence on build41 (d0cd9d73b), and the patches

### Evidence

- **The compiler test fails at HEAD.** `command_binding::tests::source_c_import_follows_the_original_command_object` fails. So the poison is intrinsic to the compiler; the LSP realm options are not the cause.
- **`run/404-trace.log`:**
  - Inside `::src`, the `proc` and `namespace` heads resolve to the builtins (`unknown=false`).
  - The export at offset 52 has compiler admission `selection=Some(Unknown)`.
  - No `ORIGINAL_NAMESPACE_BINDING_CAPTURE` line is printed. That line is printed whenever a handler walk reaches a namespace export, import or forget fact (debug builds). So the handler was never walked.
  - The very next lookup (the deferred `return`) already sees `opaque=true` and `policy=None`.

### Exact cause (static, now pinned)

1. **Admission is always Unknown.** The namespace export, forget and import subcommands use `NativeCompilationGrammar::NamedEnsembleInvocation` (namespace_.rs 996-1006). `requires_original_word_preparation` (native_compilation.rs 1982-2039) does not list that grammar, so `original_registered_compiler_preparation` returns None (ncs.rs 79-81). The fallback `spec.select_for_facts` then returns `Unknown` unconditionally for every named-invocation compiler (native_compilation.rs 3307-3309). Result: in the authoring (non-native-entry) model, a named ensemble subcommand always has admission `Unknown` and `compiled.unknown == true`.
2. **The bounded fallback is unavailable.** `walk_unknown_compiler_handler` (cb.rs 9234) is the only dispatch, and it needs `successful_handler_transfer` (ci.rs 2667). The namespace subcommands declare no `successful_handler`, so `facts.successful_handler_effects` defers to `NativeCompilationSpec::successful_handler_effects`, which returns None for the `NamedEnsembleInvocation` grammar (native_compilation.rs 2464-2491). The remaining arms also give None. So the code reaches `residual.mark_opaque_binding_mutation()` (cb.rs 9309): a global poison.
3. **Why the import's own words had no `UnknownCurrentLookup`.** The mark is applied after the dispatch point is recorded (Sol-5's observation). The pre-dispatch site cb.rs 8816 does not fire because the names are preserved.
4. **Scope.** None of this depends on the namespace. Statically, a top-level `namespace export` poisons too. Sol-5's "root export keeps Selected" claim is not supported by the code, and the patch's tests do not depend on it. The same mechanism hits every `NamedEnsembleInvocation` subcommand in authoring mode: the `array`, `binary`, `file`, `info` and `string` members that use that grammar, as well as `namespace`.

### Patch 1: `layer-a-named-namespace-statements.patch`

sha256 c9c2e56a…2836. Made with `diff -ruN a b` against `git show HEAD:`. `git apply --check` passes on both the worktree and the index. rustfmt 2024 is clean in stdin mode.

Changes:

- **`compiled_invocation.rs`:**
  - New `original_named_selection`. For a named-invocation compiler whose original lexical command is retained (`original_lexical_command`), it selects with `select_with_named_operand_view(.., NamedCompilerOperandView::OriginalArguments { selectors_retained: facts.argument_offset })`. That is exactly `select_for_facts`'s own path, minus its blanket named refusal.
  - The result for these subcommands is `NamedInvocation { lookup: ::tcl::namespace::<member>, arguments_from: 1, Direct }`, which is the C 8.6+ `CompileToInvokedCommand` shape.
  - Dependency closure is unchanged (`!dependency_closed` still forces Unknown). The late worker lookup is unchanged too (`walk_named_source_command`).
- **`command_binding.rs` `apply_namespace_transition`.** On the original-operand (source-walk) path:
  - an `Import` whose callbacks are not stock now applies a global `mark_opaque_binding_mutation`;
  - so does a `Forget` when a command observer is not quiet.

  The IR walker (no original operands) is unchanged.
- **`original_namespace_binding.rs`:** new `pattern_callbacks_are_stock(state, preload)`. It checks that `command_observers.is_quiet()` holds and, for an import, that the Registry's `NamespaceTransition::import_preload_command` slot (`auto_import`) still holds its entry binding. These are premises for n8, n9 and n10 (new probes in `probe/`). Without them, the newly precise path would be unsound.
- **Tests (command_binding.rs):**
  - `source_body_namespace_statements_keep_their_own_and_later_lookup_closed`: the 404 shape plus `incr` and `set`; the statements are not execution-unknown, and the root commands prove stock targets.
  - `source_namespace_statements_with_live_callbacks_or_imported_heads_stay_open`:
    - n8, n9 and n10: `incr x` has no proved target;
    - n1, n5 and n6: the hijacked head does not reach the stock command.
  - Primary target: the existing failing test `source_c_import_follows_the_original_command_object`.

Not compiled or run here. Run these, in this order:

1. The three compiler tests above.
2. `definition::tests::a_forget_after_the_import_stops_resolving_the_alias` (404).
3. The `source_graph` `document_sequence_*` controls.
4. The `definition::` and `workspace_index::` sets.

Watch for these risks:

- **Broader effect.** Named members of `array`, `binary`, `file`, `info` and `string` now also get `NamedInvocation` instead of poisoning. That should be more precise, but tests that encoded the old poison may move.
- **Late-lookup failure.** If the late worker lookup of `::tcl::namespace::<member>` is unknown, `walk_named_source_command` (cb.rs 9341-9344) still marks globally. Then the trace would show no `ORIGINAL_NAMESPACE_BINDING_CAPTURE` again.

Fallback if the patch is rejected: Layer A1. Add a new registry `SuccessfulHandlerSpec` variant (namespace transitions via an ensemble worker: `native_worker` gives the lookup, and the admission requires that every declared transition is a `Namespace` export, import or forget). Set it on the three subcommands (data only), together with the same premises. This only removes the poison; the statements' own words stay non-Selected, so 404 would remain red.

Also noted, and not changed: `apply_import` returning None (for example a conflict, or Jim) leaves the destination silently "closed" (cb.rs 19388-19390). That is a scoped-unknown candidate for a follow-up.

### Patch 2: `guard-test-defined-x.patch`

sha256 fe4892dc…f12a. `git apply --check` passes.

- The guard fixture gains `gets stdin x` (x is defined, with an unknown value), and the original `assert_eq!(purpose("set after", "END"), KNOWN)` stays.
- A new control uses the unset-`x` fixture: END is not KNOWN, because the document never completes normally.
- The assertion depends on `gets stdin x` keeping Selected words. `gets` has no compile hook, so it is Generic. Note that `file exists` (a named ensemble member) would poison at HEAD without patch 1.
