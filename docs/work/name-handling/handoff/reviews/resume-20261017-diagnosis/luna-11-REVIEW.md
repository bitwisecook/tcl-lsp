# Luna-11 review: Sol-11 `nextto-namespaced-definition-body.patch`

Tree: `work`. The task named HEAD 1433f2ffe. The live HEAD is 02b97c44c, which is a docs/report-only commit on top of it: `git diff 1433f2ffe HEAD -- rust/tcl-compiler/src/command_binding/` is empty. `git apply --check` passes against the live tree.

Method: static reading plus tclsh 8.6.18 / 9.0.4 / 9.1.0 probes (`probe/q1.tcl`, `q2.tcl`, `q3.tcl` beside this file). No cargo, tests, or gdb runs. I did not modify the repository.

## Verdict: HOLD. The semantics are admissible; text fixes are needed before commit.

The code change is sound relative to the walker's own lattice. What an unentered TclOO definition body can still do to the walk's command, path and export state is a subset of what an unresolved command can do. The walker already handles an unresolved command with `record_uncertainty`, without withdrawal. That precedent is exercised in `c_namespace.rs:314-358`: `opaque` before `namespace eval ::client {…}` keeps the advice and only adds `UnknownEarlierMutation`.

Two repository-rule problems block the commit as written:

- **R1:** the code comment makes a claim that tclsh refutes, and it carries development history.
- **R2:** the name-handling contract still says that unentered immediate bodies withdraw path/export advice.

Both are small text edits. Suggested text is below. Once R1 and R2 land, this is an ADMIT, subject to the orchestrator's full capture (R3).

## (1) Soundness: what `widen` covered and `record_uncertainty` does not

`AdviceGraph::widen` (`source_transition_advice.rs:1731-1750`) does five things. `record_uncertainty` (`:1722-1730`) does only the first:

1. pushes the operation to `uncertain_operations` (both do this; `record_uncertainty` deduplicates);
2. `handles.clear()`: registered-instance variable handles;
3. `withdraw_namespace_advice()` (`source_namespace.rs:29-33`): every known namespace goes into `unknown_paths` and `unknown_exports`, and `namespace_paths` is cleared;
4. drops `Descriptor` cells with non-empty lineage that are not namespace imports, i.e. renamed or moved registry commands;
5. drops `CreatedInterpreter` cells.

Because (1) is shared, every consumer that reads `!uncertain_operations.is_empty()` and turns it into `UnknownEarlierMutation` behaves the same before and after the patch. Examples: `source_class.rs:967`, `registered_instance.rs:732`, `source_instance.rs:530`, `source_publication_proposal.rs:76`, and the `earlier_unknown_mutations` sites in `logical_*`. The patch changes only (2) to (5), and only for later lookups.

### What tclsh shows a definition script can do (8.6.18, 9.0.4, 9.1.0 all agree)

- **Where it runs.** The script runs in a **new call frame** (`info level` 2 under a top-level `namespace eval`) whose namespace is the definer's: `::oo::define`, `::oo::objdefine` (also for `self {…}`), or `::oo::configuresupport::configurableclass` with path `::oo::define` for `oo::configurable` in 9.x. This matches `native_tcloo_bootstrap.rs:163-168,241-246` and Tcl's `InitDefineContext` (`TclPushStackFrame(…, fPtr->defineNs, FRAME_IS_OO_DEFINE)`).
- **Relative effects stay in the definer's namespace.** `proc made` creates `::oo::define::made`, and later class bodies can then call it as a "definition command". `namespace path ::Q` sets `::oo::define`'s path, which affects every later definition body but not the enclosing namespace. `namespace export *` and `namespace eval inner` also land in `::oo::define`. The enclosing namespace's path and exports are untouched.
- **Ordinary procs of the enclosing namespace are not callable.** `localp` gives `invalid command name`.
- **The enclosing namespace is reachable, but not only through fully qualified commands.** This corrects a premise in the brief. `uplevel 1 {…}` and `upvar 1` reach the enclosing `namespace eval` frame:
  - `uplevel 1 {namespace path ::P}` inside `oo::class create A {…}` makes the later `oo::class create B {}` in the same namespace call `::P::oo::class` (q2 case 1).
  - `uplevel 1 {proc viaUp …}` defines `::Ns::viaUp`.
  - The same works from `oo::define` and `oo::objdefine` (q2 cases 4 and 6).
  - Absolute forms such as `namespace eval ::Ns2 {namespace path ::Q}` work too (q2 case 2).
- **When the walk is already in the definer's namespace, relative effects do hit it.** With `namespace eval ::oo::define { ::oo::class create ::A {namespace path ::P}; oo::class create ::B {} }`, the second `oo::class` reaches `::P::oo::class` (q3).
- **Deferred bodies are outside this site.** `constructor`, `method`, `self method`, `forward` and `destructor` bodies run later and are handled by the constructor-call path (`source_constructor_effects.rs`). The patch does not change that path.

### Conclusion

Each surviving effect (`uplevel`/`upvar` into the caller frame, absolute names, and relative effects when current is the definer namespace) is also available to an unresolved command. An unknown proc runs in its own frame, can `uplevel 1`, can use absolute names, and can live in the current namespace. So the patch's claim, "leaves the residual of an unresolved command", is correct.

The **stated reason** in the comment is wrong in two ways:

- "never this walk's current one" is false (q3);
- the comment omits that the frame's caller is the current frame (`uplevel 1`).

After the patch, a pathological `uplevel 1 {namespace path …}` in a class body gives a later relative head a wrong slot. The only signal is the existing `UnknownEarlierMutation` obligation, which is exactly what an equivalent unknown command already gets.

Before and after for the effects that `widen` withdrew, all carried as uncertainty only after the patch:

| Effect | Before | After |
|---|---|---|
| Relative head after a body inside `namespace eval` | refused (`c_namespace.rs:70`, `logical_navigation.rs:537`, `source_transition_advice.rs:1635`) | resolves through local slot, then known path, then root |
| Earlier `namespace path` of any namespace | cleared | kept |
| `rename`d registry descriptors | dropped | kept |
| Registered-instance handles | dropped | kept |
| `interp create` children | dropped | kept |

## (2) Registry-data only

Yes, in the sense the rule requires. The gate reads `ResolvedInvocation::authored_source_definition_body_grammar()` (`tcl-registry/src/resolved_invocation.rs:5561`, `pub`) and `DefinitionBodyGrammar.family` (`definer.rs:1848`, `pub`). It contains no command names.

`graph.resolve` (`source_transition_advice.rs:1881-1895`) returns only `Descriptor` cells, so only registry specs reach this site. A source-defined metaclass `Meta create X {…}` is already unresolved, so it already goes through `record_uncertainty` at `body_effects.rs:1233`.

The stock TclOo grammars are `TCLOO_GRAMMAR`, `TCLOO_ABSTRACT_GRAMMAR` and `TCLOO_CONFIGURABLE_GRAMMAR`, used by `oo::class`, `oo::define`, `oo::objdefine`, `oo::abstract`, `oo::singleton` and `oo::configurable`. All of them run the script in a definer namespace.

**F3 (low):** the SpecTcl loader defaults `family` to `DefinerFamily::TclOo` for any `definition_body {…}` block that lacks a `family` line (`tcl-spectcl/src/loader.rs:5093-5095`; the override is at `:5122-5134`). A user or vendor pack that describes a home-grown class system with `namespace eval $name $body` or `uplevel 1 $body` semantics would therefore also take the `record_uncertainty` path, even though its body really runs in the current namespace.

- Today no bundled pack does this: `specs/*.tclspec` has no `definition_body`, and the Jim surface declares `family JimClass`.
- The residual is still bounded by the unresolved-command residual.

This needs no change for admission. Optional hardening: also require the stock `tcl` pack provenance, or a registry definition-namespace datum. The latter would be a `CommandSpec` extension, not a name match.

## (3) Blast radius

Sol-11's sample covered 25 `tcl_lsp_core` and 36 `tcl_compiler` lib tests that mix `namespace eval` with non-empty class bodies, using an exact gdb emulation of the redirect. It found zero verdict or message differences.

That sample is narrow. A static scan (`tcloo_body_tests.tsv`) found **1805** `#[test]` functions in tcl-compiler, tcl-lsp-core and tcl-lsp-server that contain a non-empty TclOO definition body. Of those, **227** also contain `namespace eval`. **181** of the 227 were not in Sol's sample (`ns_candidates_unsampled.tsv`). The unsampled set includes:

- all of `tcl-compiler/tests/analyser.rs` (42), e.g. `a_relative_superclass_does_not_cross_link_to_another_namespace`, `a_relative_superclass_prefers_its_own_namespace_over_a_decoy`, `computed_creation_does_not_carry_proc_locals_across_namespace_eval`, and `a_rename_written_after_the_creation_call_does_not_apply`. In these, the second and later relative `oo::class create` in a namespace now goes through the tape's `source_class_declaration_at` instead of `defer_class_creation`. That is a real code-path change.
- `tcl-lsp-core/tests/*.rs`: `name_resolution`, `references_rename`, `references_residual`, `oo_mro_parity`.
- every `tcl-lsp-server/tests/e2e/*`: `tcloo_navigation` (7), `issue923_class_refs` (4), `issue923_crossdoc` (3), `issue1296_metaclass_chain` (3), `recovery` (7), `definition`.
- `workspace_index::{resolve_wildcard_import_resolves_exported_class_cross_document, oo_forward_target_is_a_reference_to_the_command}`.
- `type_hierarchy::{prepare,subtypes}_disambiguate_same_name_across_namespaces` and `supertypes_resolve_bare_namespaced_base_owner_aware`.
- `implementation::namespaced_*`.

Expected flips:

- **Positive:** the two target tests, `definition.rs:7114` (`jump_to_nextto_relative_qualified_class_method`) and `:7127` (`jump_to_nextto_fully_qualified_class_method`). Sol's emulation shows both pass.
- **No flip, per Sol's emulation:** the group A logical hierarchy failures.
  - `type_hierarchy` logical source selection, logical workspace and conditional relation still fail with identical messages. For example, `type_hierarchy.rs:1183` has left `[]` and right `["::N::Base"]`.
  - `implementation` `use_resolved_bare_relations_in_nested_namespace` (`implementation.rs:585`: `[2]` vs `[2,3,4]`) and `do_not_join_equal_tail_class_owners` still fail.
  - `rename_safety` `fp_refuses…` and `fn_guard…` still fail with "metadata reference inventory is unavailable".
  - So the patch is not their root cause, and it should not be credited with fixing them.
- **Possible negative flips:** tests that assert a refusal after a non-empty class body. The likeliest are TN/FP guards whose answer used to come "for free" from the withdrawal and must now come from real disambiguation:
  - `issue923_class_refs::fp_same_named_class_in_another_namespace_not_cross_linked`
  - `tcloo_navigation::tn_an_inner_namespaced_class_shadows_the_global_one`
  - `tcloo_navigation::tn_consumer_bare_dispatch_is_attributed_to_its_own_namespaces_class`
  - `references_rename::rename_classmethod_does_not_rewrite_a_sibling_namespaces_dispatch`
  - `analyser.rs::a_relative_superclass_does_not_cross_link_to_another_namespace`
- **Global scope is affected too, not only `namespace eval`.** At `::`, path withdrawal hardly matters because local and root are the same slot. Effects 2, 4 and 5 still change, though, e.g. `rename puts p; oo::class create A {…}; p x` now resolves `p`.

**R3:** run the full tcl-compiler, tcl-lsp-core and tcl-lsp-server capture against baseline before admitting. Gdb emulation of one site does not replace a compile.

**R4 (recommended):** add a compiler regression test at the `c_namespace.rs` / `source_class.rs` test level. A second relative `oo::class create` after a non-empty body inside `namespace eval` should get `source_class_declaration_at`, with the `UnknownEarlierMutation` obligation present. Add a companion test where `uplevel 1 {namespace path …}` in the body keeps that obligation, which pins the soundness boundary.

## (4) Compile, format and style risk

- **Signatures match HEAD.**
  - `record_uncertainty(&mut self, original: &[NativeWord])` and `widen(&mut self, original: &[NativeWord])` have identical argument types, and `&native.words` is already passed at `body_effects.rs:1207/1222/1233/1245/1273`.
  - `schema` is a `tcl_registry::ResolvedInvocation` that is only borrowed above, so it is not moved.
  - `DefinerFamily` derives `PartialEq, Eq, Copy` (`definer.rs:1540`), and `tcl_registry::definer` is `pub mod` (`lib.rs:77`).
  - The fully qualified path `tcl_registry::definer::DefinerFamily::TclOo` is already used in this file (`body_effects.rs:1422`).
- **Formatting.** I applied the patch to a scratch copy and ran `rustfmt --edition 2024 --check`: clean.
- **Lints.** No `#[allow]`. The inner `if/else` inside the outer `if` is not `collapsible_if`, because it has an `else`. I see no new clippy::pedantic exposure.

## Required text fixes

**R1** — replace the comment at `body_effects.rs` (patched lines 1297-1304) with something like the following. It states current semantics without history, and it is correct for `uplevel` and for configurable and objdefine:

```rust
            // A `TclOO` definition script runs in a new frame in its definer's
            // namespace (`::oo::define`, `::oo::objdefine` or a configurable
            // support namespace), and the definition-member inventory owns its
            // members. Its remaining reach into this walk — `uplevel`/`upvar`
            // to the caller frame and absolute names — is that of an
            // unresolved command, so it records the same uncertainty instead
            // of withdrawing every namespace's path and export advice.
```

Remove the trailing clause "— which refused each later relative head (`oo::class create B …`) in an enclosing `namespace eval`". That clause is development history (AGENTS.md § Documentation).

**R2** — `docs/design/contracts/name-handling.md:152-156` says "Possible child binding changes and unknown namespace path/export effects still withdraw subsequent source advice", and `:201-204` says "Immediate script bodies … keep their existing effect projection". Add one sentence such as:

> "An unentered `TclOO` definition body (a `DefinerFamily::TclOo` definition-body grammar) runs in its definer's namespace in a new frame, so it records the unresolved-command uncertainty instead of withdrawing path/export advice."

Cite the tclsh 8.6.18 / 9.0.4 / 9.1.0 evidence (q1–q3). Optionally, record the probes in the `experiments/name-handling` catalogue.

## Out-of-scope observation (pre-existing, not caused by this patch)

`AdviceGraph::unknown_exports` is written (`source_namespace.rs:30,123`, `body_effects.rs:922,926`) but never read. `NamespaceTransition::Import` (`source_namespace.rs:130-200`) filters through `namespace_exports`, which `widen` never clears. So the export half of `withdraw_namespace_advice` has no effect even before this patch, and the patch's export-side change is therefore inert.
