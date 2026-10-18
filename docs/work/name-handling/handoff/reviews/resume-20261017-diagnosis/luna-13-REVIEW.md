# Luna-13 review of Sol-13's two patches

- **Baseline:** HEAD 16e915a18. It is docs-only over d0cd9d73b, and every reviewed source file is byte-identical to the `la/a` and `gt/a` trees.
- **Method:** static reading, plus `nice tclsh8.6` and `tclsh9.0` probes in `probe/` (`results.txt`, `guard-results.txt`, `prior-rerun.txt`). Nothing in /home/user/tcl-lsp was modified, and no cargo was run.
- **Line references:** all lines are HEAD lines. `cb.rs` means `rust/tcl-compiler/src/command_binding.rs`. `ci.rs` means `.../command_binding/compiled_invocation.rs`. `onb.rs` means `.../command_binding/original_namespace_binding.rs`. `nc.rs` means `rust/tcl-registry/src/native_compilation.rs`.

| patch | verdict |
|---|---|
| layer-a-named-namespace-statements.patch (sha256 c9c2e56a…2836) | **HOLD** |
| guard-test-defined-x.patch (sha256 fe4892dc…f12a) | **ADMIT** (two optional follow-ups) |

---

## Patch 1: layer-a-named-namespace-statements: HOLD

The selection change is sound in shape. The new premise helper `pattern_callbacks_are_stock` is not: it inverts the existing seal's premise on `auto_import`, and it omits two other seal premises. As a result:
- it admits real hijacks (tclsh-proved);
- it would poison the very case the project already models as closed, which breaks five existing tests.

### P1-1 (blocker): the auto_import premise is inverted

**What the patch does.** Patch `onb.rs` new lines 24-51: `pattern_callbacks_are_stock(state, true)` accepts an import when the global preload slot's current binding equals its initial binding (`current == original_initial_bindings_for_key`). In the authoring baseline, `auto_import` is a present Registry command (`rust/tcl-registry/src/commands/tcl/auto_import.rs:31-60`, `runtime_backing: package_source("init.tcl")`, `Traits::LOADS_EXTERNAL_UNIT`). So the helper means "stock auto_import present is fine". That contradicts the project's own model in three places:
- The existing precise seal requires the slot to be **Missing** (`onb.rs:669-677`, `import_is_fresh_and_closed`).
- Every positive import fixture first runs `rename auto_import savedPrelude`. There are 8 occurrences, for example `source_command_world.rs:3117` and `3167`, `onb.rs:823`, and `original_command_mutation.rs:334` and `384`.
- `cb.rs:22466-22486` (`external_unit_execution_widens_direct_and_embedded_command_bindings`) asserts that `auto_import pkg::*` makes the domain opaque, because "unavailable external code may mutate any command binding". `namespace import` calls exactly that helper (C `TclImport` → `Tcl_EvalObjv(auto_import pattern, TCL_GLOBAL_ONLY)`).

**Counterexamples (tclsh 8.6 and 9.0, `probe/results.txt`).** Both have an unchanged auto_import slot, and the patch admits both:
- **n11** (`n11_stock_auto_import_index.tcl`): `set ::auto_index(::m::zz) {rename ::incr ...}` and then `namespace eval ::a {namespace import ::m::*}`. Stock `auto_import` runs `namespace inscope :: $auto_index($name)`. Result: incr is hijacked and after=0. The control n11c has no index entry and gives after=1.
- **n15** (`n15_auto_path_tclindex.tcl`): `lappend ::auto_path $dir`, where `$dir/tclIndex` indexes `::m::zz`, then the same import. Stock `auto_import` → `auto_load_index` sources the external unit. Result: incr is hijacked and after=0. This is the common `lappend auto_path` idiom.

**Predicted regressions (static).** The converse also holds: after `rename auto_import savedPrelude`, current (`{Missing}`) differs from initial (Present). The helper then returns false, and patch `cb.rs` +19379-19384 calls `state.mark_opaque_binding_mutation()`, which also `original_command_world.withdraw()`s (`cb.rs:14452-14460`). These tests run their Direct-mode source walk through `transfer_native_invocation` → `apply_declared_binding_transitions(.., Some(&original_operands))` (`cb.rs:5563-5571`), so the new check runs for them. Each expects `original_completed_command_world()` to be `Some`, and is predicted to flip to FAIL:
- `source_command_world::tests::original_qualified_forget_joins_current_origin_after_source_and_alias_moves` (`source_command_world.rs:3114`)
- `source_command_world::tests::original_namespace_import_keeps_the_source_declaration_allocation` (`source_command_world.rs:3164`)
- `original_command_mutation::tests::original_imported_command_move_keeps_the_wrapper_and_direct_origin` (`original_command_mutation.rs:330`)
- `original_command_mutation::tests::original_imported_command_move_keeps_origin_and_observer_barriers` (`original_command_mutation.rs:374`)
- `original_namespace_binding::tests::original_namespace_import_and_forget_keep_actual_origin_with_closed_prelude` (`onb.rs:817`)

**Required.** Require the preload slot to be exactly `{Missing}`, or else widen globally. Factor the preload loop out of `onb.rs:669-677` into one shared function that both the seal and the transition call. If the orchestrator wants a stock auto_import to be admissible, that needs its own Registry-declared premise, all of which must hold:
- `::auto_index`, `::auto_path` and `::tcl::auto_oldpath` have no document or dynamic writes;
- the entry environment is closed;
- the premise is backed by a tclsh proof.

Until then, the n11 and n15 refusals must be tests.

### P1-2 (blocker): `import -force` replacement cleanup is not covered

The seal also requires every destination slot to be fresh (`onb.rs:752-758`; comment at `onb.rs:455-456`: "invokes no replacement cleanup"). The helper checks only `command_observers`.

**Counterexample (tclsh 8.6 and 9.0).** **n12** (`n12_force_destroys_object.tcl`): a TclOO object `::a::h` whose class destructor rebinds `::incr`, then `namespace eval ::a {namespace import -force ::m::*}`. The destructor runs: incr is hijacked and after=0. `apply_import` (`onb.rs:236-254`) only replaces the bindings.
- Controls: n13 (coroutine killed by `-force`) and n14 (forget of an imported object) stay stock.

**Required.** Refuse a replaced destination binding that can own cleanup (object or class instances), or mirror the seal's fresh-destination premise.

### P1-3 (major): the premise set is weaker than the seal's

The helper uses `command_observers.is_quiet()` only. The seal's `quiet()` (`onb.rs:339-346`) also requires all of the following:
- `!has_opaque_domain`;
- `!unknown_entry`;
- `!source_step_observed()`;
- `!source_execution_observed(None)`.

For forget, the seal also requires `import_origins_are_live` (`onb.rs:624`, called at `onb.rs:470`), which the patch omits. Sol's own Layer-B design (NOTES, premise 3) names `quiet(incoming)`.

**Required.** Reuse `quiet()` and `import_origins_are_live`, or justify each omission in a comment. Keep a single source for these premises, so that the seal and the transition cannot diverge again.

### P1-4 (major, pre-existing gap now reaching bodies): nothing proves the ensemble map is stock

Question (b) asks what proves the map is stock: in authoring mode, nothing does.
- **The map check passes unconditionally.** `original_ensemble_mapping_holds` (`ensemble_compilation.rs:123-140`) returns `true` whenever `baseline.native_entry.is_none()` and the target label is the ensemble. It never inspects the map, `-unknown`, `-subcommands` or `-parameters`.
- **Reconfiguration is barely recorded.** `namespace ensemble configure|create` records only `NamespaceTransition::Ensemble { namespace: Current }` (`namespace_.rs:779-800`). Its consumer only inserts *that* namespace into `unknown_lookup_namespaces` (`cb.rs:19433-19438`), and that set refuses only relative heads inside it (`cb.rs:15625`).

**Counterexamples (tclsh 8.6 and 9.0).**
- `e1b_literal_map_in_body.tcl`: `namespace eval ::z {namespace ensemble configure ::namespace -map {eval ::tcl::namespace::eval export ::hijack}}`, then `namespace eval ::a {proc h {} {}; namespace export h}`. Result: incr is hijacked and after=0.
- `e6b_string_literal_map_in_body.tcl`: the same for `::string -map {repeat ::hijack}`.
- `e3` (via `-subcommands` plus `-unknown`) is also hijacked.

**Classification.**
- At HEAD, named members in bodies poisoned globally, so they were accidentally sound. With the patch they select `NamedInvocation` of the stock worker, which is a sound→unsound flip for e1b and e6b.
- Root Direct dispatch and inline members (e9: `namespace current` remapped) already ignore the map. So this is a model-wide gap that the patch extends; it is not new in kind.

**Required.** Add e1b and e6b as tests, and either:
- treat `Ensemble{Create|Configure}` on a named ensemble as a global binding mutation, unless it is the `exists` or query form; or
- record a `GAPS` / tracking entry in the active coordination docs and hold the patch until it is decided.

**Worker redefinition is already covered.** These variants are refused statically through `implementation_dependency` (`ci.rs:2158-2183`: registry_backed, generation 0, label match) and the late worker lookup:
- e4: rename of the worker;
- e5: proc redefining `::tcl::namespace::export`;
- e11: renaming a proc into the worker slot;
- e10: the same change mid-chunk.

A renamed public ensemble (e8) stays stock at runtime, which is consistent.

### P1-5 (major): correction to Sol-13's diagnosis (addendum point 4)

Sol-13's addendum says "a top-level `namespace export` poisons too … Sol-5's claim is not supported by the code". **That is incorrect.**
- **The root is Direct.** `authoring_native_compilation()` uses `mode: Direct` (`rust/tcl-compiler/src/environment_ingress.rs:62-73`), and `select_invocation` returns `CompiledInvocationSelection::default()` in Direct mode (`ci.rs:1017-1019`). Root commands therefore never reach `registered_compiler_selection`, so they never get the named `Unknown`.
- **The poison is body-specific.** Only compiled (BytecodeObject) bodies, such as `namespace eval` bodies and the bodies within them, take the path at `ci.rs:1244` → `1376-1388`.

**Consequences.**
- *Blast radius.* Only named members inside compiled bodies change. That is a large set: `array names/get/size/...` (`array_.rs:204-217`), the `info` named members, the `file`, `binary` and `string repeat` members, and the `namespace children/exists/parent/delete/path/unknown` members. At HEAD these already poisoned globally, or took the bounded `successful_handler` route where one is declared (`string_.rs` and `binary_.rs` declare `successful_handler`; `array_.rs:323` declares `PossibleBodies` for `array for`).
- *Route change for `string repeat`, `binary` and `array for`.* These members switch from the bounded handler transfer to `walk_named_source_command`. `array for` (9.0+, ScriptObject body) must still have its body visited through the worker facts; that needs a test.
- *Root precision.* The patch's helper also runs on root Direct imports. It makes them *more* conservative for n8, n9 and n10, which is good, but the P1-1 inversion applies there too.

### P1-6 (registry rule)

**(a) The worker name is Registry data. PASS.**
- `::tcl::namespace::<member>` is the `slot` of the static `NativeCompilerImplementationLookup` table `NAMESPACE_NAMED_LOOKUPS` (`namespace_.rs:926-990`). It is wired by `namespace_named_compilation` (`namespace_.rs:992-1006`).
- The `array`, `file`, `binary`, `info` and `string` lookups are declared the same way (`array_.rs:204-217`, `file_.rs:53-67` with `concat!` in the registry, `binary_.rs:42-50`, `info_.rs:186-220`, `string_.rs:1766-1777`).
- `named_ensemble_grammar` returns that `lookup` reference (`nc.rs:4662-4693`). The patch builds no string and names no command.

**Smell.** The patch bypasses a Registry-level refusal. `select_for_facts` deliberately returns `Unknown` for every named compiler (`nc.rs:3307-3309`), and the consumer now calls the lower-level `select_with_named_operand_view` to skip it. The decision "named compilers are selectable when the original lexical command is retained" belongs in the Registry. Two ways to put it there:
- add `NamedEnsembleInvocation` to `requires_original_word_preparation` (`nc.rs:1982-2039`), so that `original_native_compilation` (`native_compilation_source.rs:75-125`) produces the real `NamedInvocation` plan, which exists (`native_instruction_plan.rs:313-336`);
- add a Registry method for the original-facts selection.

**Side effect of the bypass.** Because `original` is None, `result.structured` is None and the named `preparations` become empty (`ci.rs:1300`). Compiled preflight then sees no named preparation steps (`compiled_preflight/original_preparation.rs:169`). That is a literal and local fidelity gap that the Registry route would avoid.

### P1-7 (compile, fmt, attributes): OK

- Every API used exists with matching signatures:
  - `CompilerInvocation` is `Copy` (`ci.rs:1217`);
  - `InvocationWords` is `Copy`;
  - `original_lexical_command(words, offset, config)` is as at `ci.rs:1640`;
  - `select_with_named_operand_view` is at `nc.rs:2529-2536`;
  - `is_named_invocation_compiler` is `pub const` (`nc.rs:3188`);
  - `original_registry_command_paths` returns `Option<Vec<Vec<SourceCommandKey>>>` (`original_command_table.rs:1908-1913`);
  - `original_initial_bindings_for_key` is at `original_command_table.rs:2829`;
  - `command_observers.is_quiet()` is `pub(super)`, already used at `onb.rs:345`;
  - `tcl_registry::InvocationFacts` is re-exported.
- `rustfmt --edition 2024 --check` is clean on all three `la/b` files.
- There is no `#[allow]`.
- `git apply --check` passes, and the `la/a` trees equal HEAD.

### P1-8 (tests)

**Tests that may flip.**
- *Predicted FAIL.* The five tests in P1-1.
- *Intended PASS.* `source_c_import_follows_the_original_command_object` (`cb.rs:21344`). Its fixture has **no** `rename auto_import`, so under the corrected P1-1 premise it stays red. Its expectation conflicts with the project's auto_import model. The orchestrator must decide: either the fixture gains `rename auto_import savedPrelude;` like the other 8 positive fixtures, or the stock-auto_import premise above is designed and proved.
- *The LSP 404 fixture.* `definition::tests::a_forget_after_the_import_stops_resolving_the_alias` and its relatives get the same decision.
- *Unaffected.* The IR-walker tests (`cb.rs:22387-22460`, `ModuleCommandBindings::analyse`).

**Problems with the patch's own tests.**
- `source_body_namespace_statements_keep_their_own_and_later_lookup_closed` asserts the unsound expectation (an import with stock auto_import proves root `incr`). Its fixture needs `rename auto_import savedPrelude;`.
- `source_namespace_statements_with_live_callbacks_or_imported_heads_stay_open` is correct for n1, n5, n6, n8, n9 and n10. I re-ran all six in both tclsh versions (`probe/prior-rerun.txt`): n1 and n5 have hijacked heads, and n6 and n8-n10 give after=0. It must add n11, n12 and n15 (refused), and e1b and e6b (refused, or a documented gap).

**Required to re-admit.** Do P1-1, P1-2 and P1-3, decide P1-4, update the tests as above, and correct NOTES addendum point 4.

---

## Patch 2: guard-test-defined-x: ADMIT

**tclsh confirmation (`probe/guard-results.txt`, 8.6 and 9.0).**
- The original fixture (g1) and the control (g3) both fail with `rc=1 can't read "x": no such variable`, and after=0. The document never completes normally, and `set after` is never reached. END being withheld is therefore correct and conservative. KNOWN would only be vacuously true, so the original assertion was a precision demand the walk cannot meet, not a soundness requirement.
- The new fixture with `gets stdin x` (g2) gives:
  - stdin `1`: after=1, branch=1;
  - stdin `0`: after=1, branch=0;
  - stdin empty, EOF or `abc`: `expected boolean value` error.

  So the branch is genuinely conditional, and the document can complete normally.

**This is not a weakening.**
- Every original assertion is kept, including `purpose("set after","END") == KNOWN`, and the `namespace eval` / `if` / `proc` structure (the suffix stopper) is unchanged.
- Only the accidental definitely-missing read is removed. That read made the guard meaningless.
- The control pins the unset case separately.
- The patch is independent of patch 1: `gets` is Generic, and the root runs in Direct mode.

**Optional.**
1. Rename the closure `unset` (for example to `missing`), and comment that withholding is the sound answer there, not a semantic requirement.
2. Comment in the fixture that `if` may still error on a non-boolean `x`, so a *possible* error is deliberately not an early exit. Alternatively, use an always-boolean unknown such as `set x [expr {rand() < 0.5}]`.

**Follow-up (same defect, separate change).** `document_sequence_ends_at_a_possible_early_exit` (`rust/tcl-lsp-core/src/source_graph.rs:3380-3381`) uses `if {$x} return` and `if {$x} {exit 1}` with `x` unset. Those cases pass because the read fails, not because of the `return` or `exit`, so they do not test what they claim. Add `gets stdin x\n` to both.
