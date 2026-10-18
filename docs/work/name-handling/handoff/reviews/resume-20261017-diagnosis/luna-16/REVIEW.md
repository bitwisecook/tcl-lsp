# Luna-16 review of Sol-17 (`lsp-package-standalone-control-role-residue`)

- **Reviewed:** `sol-17.patch`, `parts/{A,B,C,D,R}-*.patch`, `NOTES.md` and `probe.tcl`, against HEAD `75c75665f` on branch `work`. None of the touched files changed after `a146f2608`.
- **Method:** static reading only, plus `nice tclsh8.4/8.5/8.6/9.0` probes. The probes are in `probe.tcl` and `probe84.tcl` in this directory.
- **Not done:** nothing was compiled or run in the repo.

## Verdicts

| Part | Verdict | One line |
|---|---|---|
| A realm widening | **ADMIT (conditional)** | The lookup is exact and never returns a different word. NOTES under-declares who calls it: analyser production paths also change. |
| B `Condition::holds` grammar → Inert | **ADMIT** | Fails closed and uses the shared `tcl_dialect` owner. An 8.4 arity gap of the same kind is left open (follow-up). |
| B′ dead `Condition::TclSatisfies` | **ADMIT** | Nothing constructs it outside `cfg(test)`, and it has no consumers in the workspace. |
| C ordered `if` conditions | **HOLD** | Its soundness depends on D (or a replacement). The predicate checks selection count, not evaluation timing. User math functions lose coverage they only had by accident. |
| D new branch keys → Shadowed | **HOLD** | No completeness is inferred from absence. But a Shadowed cell *occupies* the slot, which withdraws logical-navigation header candidates for procs defined inside `namespace eval`, `if` or `catch` bodies. High regression risk. |
| R registry effects | **ADMIT** | Checked against the C sources for 8.4, 8.5, 8.6, 9.0 and 9.1. No codegen, callback-inventory or SpecTcl parity drift. Barrier consumers move as intended. |

The 180 s nested-wrapper path is untouched (see the last section).

---

## A — `realm.rs::executed_script_for_word` widening

**Patch site:** `rust/tcl-compiler/src/realm.rs:599-616`.

### Exactness: sound

- The second probe fires only after an exact miss. It keeps `span.start()` and moves the end to `word_closer_offset_at(span)+1`, which is the closer of *the same* word (`tcl-lexer/src/ranges.rs:543-585`).
- Carriers are keyed `(root_origin, (start,end), element)` (`command_binding/origin_inventory.rs:820-829`, `894-907`). Two distinct written words in one origin never share a start offset, so the widened key can only hit the same word's carrier.
- The image used is `bindings.source_origin()` = `root_origin` (`origin_inventory.rs:292`). That is the same origin the lookup key uses, so coordinates agree. An out-of-range span returns unchanged and so misses.
- Conflicting multi-records still return `None` (`origin_inventory.rs:826-829`).
- Element-keyed (list-element) carriers are not reachable, because the lookup passes `element=None`.

| Word shape | Behaviour |
|---|---|
| Braced | Exact key = token span = recorded key (`word_expr.rs:254`). Unchanged. |
| Bare | `closer_for` fails, so the span is unchanged. |
| Quoted | Widened to the `Template` whole-word span (`word_expr.rs:213-233`). Hit. |
| Quoted with welded tail (`"a"b`, non-strict 8.x) | `original_word_span` ≠ closer+1, so it misses. Fails closed. |
| Expanded | No body carrier is recorded for them; a stray widening still could only reach the same word. |

### Finding A-1 (medium; NOTES claim is false)

NOTES says the realm-level method is "called only by the survey and by the `registry_invocation` tests". In fact `CommandBindingRealm::executed_script_for_word` is also called from **production analyser paths**:

- **`analyser/handlers.rs:2257`** (proc-body param-trait scan, `head_identities: CommandBindingRealm`, `analyser/state.rs:935`). Quoted (`proc p {} "…"`) and `${v}` proc bodies now obtain an `executed_source`.
- **`analyser/commands.rs:446-449`** (`analyse_list_quoted_body`). `body_tok` is a `TokenType::Cmd` token whose span is inner-end (`[list a b`, see `ranges.rs:487-495`). The interpreter records `WordExpr::CommandSubstitution` under the component span, which includes the brackets (`word_expr.rs:418-432`, `executable_parts.rs:56`).
  - Before this patch, the lookup could never hit.
  - After it, `if 1 [list …]`-style list-built bodies gain `evaluated_body_invocation`.
  - This is probably the intended design finally working, but it is an unvalidated behaviour change.

### Finding A-2 (low; consistency)

Lowering keeps calling `SourceCommandBindings::executed_script_for_word` with raw token spans at these sites:

- `lowering/mod.rs:4274`, `5222`, `6281`
- `lowering/structured.rs:1525`, `1592`

So for quoted, `[…]` and `${}` body words the analyser now sees a carrier that lowering does not. Prefer one key convention at the owner: normalise at `record_executed_word` / `executed_script_for_element` in `origin_inventory.rs`. That keeps analyser and lowering in agreement. It also still satisfies the compiler test that compares against the realm.

### Conditions for admission

- Add the analyser selections to the validation run: `analyser::param_traits`, list-quoted body / `analyse_list_quoted_body` users, and the `registry_invocation.rs:12042/12083` tests.
- Correct the NOTES caller claim.

---

## C — `body_effects.rs` ordered first-match conditions

**Patch site:** `body_effects.rs:1042-1104` (patched numbering).

### Evaluation order (oracle-checked)

`if {[a]} … elseif {[b]} {B2}` runs `a`, then `b`, then `B2`, and `B2` sees `a`'s redefinition. The probe output is `order: a b B2:redefined-by-a` on both 8.6 and 9.0.

The patch matches this order:

- Conditions are walked first, in `apply_immediate_body_effects` (`body_effects.rs:641`).
- Regions start from `original = graph.clone()` after all conditions (`body_effects.rs:745`).
- Children are threaded in lexical order, across `Expr` roles in role order.
- For first-match, no condition is evaluated after any body.

Starting every region from the state after *all* conditions over-approximates `B1`. That is sound.

Failure paths still fail closed:

- `expressions` → `None` → `graph.widen` (`body_effects.rs:1289-1291`), or `record_uncertainty` under inventory (`:652-656`).
- `widen` pushes the operation (`source_transition_advice.rs:1731-1732`).

### Finding C-1 (high; dependency on D)

The removed parent marker was the only thing covering commands *created* by a condition substitution. `retain_possible_effects` (`body_effects.rs:883-904`, pre-patch) drops keys that are new in the branch.

**Oracle (8.6 and 9.0):**

```
if {[namespace eval ::ns2 {proc package args {}}; list 1]} {namespace eval ::ns2 {package ifneeded viacond 1.0 {}}}
```

registers nothing. Under C alone, the graph after the condition has no `::ns2::package` cell, so the body's `package` falls back to the global builtin. That is a false registration, where before it was blocked by the marker.

The same gap already exists for clause *bodies*, so C widens an existing hole to conditions. C must not land without D or D's replacement.

### Finding C-2 (medium; wrong registry predicate)

`ClauseSelection::FirstMatch` means "the first clause matches" (`tcl-registry/src/clause_grammar.rs:205-212`). It does not mean "each condition is evaluated at most once and before every body".

- The SpecTcl loader defaults `selection` to `FirstMatch` (`tcl-spectcl/src/loader.rs:4783`). A user `.tclspec` loop-like command with `Expr` slots and `PerIteration` / `LoopFixture` / `Always` / `Protected` rows would be threaded as ordered.
- **Fix:** derive the predicate from the timing data the registry already has: `FirstMatch` *and* head/rows/tail all `ClauseTiming::Selected` (true for `if_.rs:86-104`). Ideally expose it as a `ClauseGrammarSpec` method, so the consumer does not reinterpret selection.

### Finding C-3 (medium; widened pre-existing gap)

`checked_*_expression_substitutions` returns only variables and `[…]` (`tcl-syntax/src/expr/substitution.rs:183-225`). User `tcl::mathfunc::*` calls are therefore not walked by `BodyWalk`.

- Before the patch, `if {[x] && f()}` was covered only incidentally, by the marker that `[x]` triggered. After it, the marker is gone.
- **Oracle:** a `tcl::mathfunc::f` that defines `::ns::package` inside `if {[string length x] && f()}` suppresses the registration (`mathfunc-shadow: {}`).
- **Fix:** keep the marker whenever the checked expression contains a function call that is not a registry-closed builtin math function, using `OriginalSourceExpression::function_calls()` and the registry's mathfunc specs. Variable reads already keep the marker (read traces), which is correct.

### Not affected

- Loops (`while_.rs:43`, `for_.rs:52`, `foreach`, `lmap`, `dict`, `array`, `catch` are `All`) keep the exact old code path. That includes the single pre-loop clone and the marker.
- `try` is `FirstMatch` but has no `Expr` slot, so it is unaffected today.

---

## D — `retain_possible_effects` new keys → `Shadowed`

**Patch site:** `body_effects.rs:905-911`.

### Soundness: no absence-as-completeness

- A `Shadowed` cell is strictly more conservative than an absent one for resolution: `blocks_registry_source` blocks (`source_transition_advice.rs:1751-1777`), `resolve` returns None, and `lookup_head_key` under a procedure namespace keeps the local key (`:1610-1619`).
- In the survey, a refused control sets `complete=false` and `Undecidable` (`reachability/original.rs:262-275`).
- Cells are only inserted by transitions. There is no lazy read-side materialisation that would turn a mere *call* inside a branch into a shadow (`native_baseline.rs:32-73`, transition inserts only).
- **Oracle for T045:** `explicit-ns: {}`, `default-global: 1.0` (8.6 and 9.0).

### Finding D-1 (high; regression risk not analysed in NOTES)

The `Shadowed` cell is not just a resolution barrier. It **occupies the slot** for logical navigation:

- `external_header_candidates_at_selected` returns `None` as soon as `graph.cells.contains_key(selected)` (`logical_navigation.rs:388-401`).
- Before this patch, a proc defined inside any body region (`namespace eval ::A {proc helper …}`, `if {…} {proc foo …}`, `catch {proc …}`) left no cell after the merge. `capture_document` (`logical_navigation.rs:241-265`) then offered `::A::helper` as an external header candidate.
- That candidate feeds:
  - `WorkspaceIndex::logical_declaration_targets_for_call` (`tcl-lsp-core/src/workspace_index.rs:4613`, also `5012`, `5184`, `6378`)
  - `analyser/handlers.rs:10469`
  - `project_procedure_roles.rs:365`
  - `source_relation_provider.rs:745`
- After D, the merged `Shadowed` cell withdraws those candidates. That removes definition, references and workspace targets for the most common Tcl layout: procs inside `namespace eval` and called later at top level.
- NOTES' statement that the only change is that "a namespace-local definition … now shadows a global fallback" is therefore incomplete.

**Recommended replacement.** Record branch-created keys as *possible occupants* that block falling through to a *later* lookup candidate, without becoming a cell:

- Add a separate set consulted by `lookup_logical_head_key`, `lookup_head_key` and `blocks_registry_source`.
- Alternatively, insert `Shadowed` only where the key is a non-final candidate whose fallback resolves to a registry, baseline or descriptor meaning.

This keeps T045 (`::ns::package` vs `::package`) and C-1 closed and leaves `::A::helper` navigation intact.

**Must run before admission:**

- `tcl-lsp-core` `workspace_index` tests (`package_provider()` / `bystander` `puts [::mymod::helper]`, `:11474-11700`)
- logical navigation and definition/references tests
- `tcl-compiler` `logical_navigation`, `source_relation_provider` and `project_procedure_roles` tests

---

## R — registry: `package provide` / `package vsatisfies`

**Patch sites:**

- `rust/tcl-registry/src/commands/tcl/package_.rs:478-480`: `provide` → `PACKAGE_REGISTRATION_EFFECTS`, which is PackageState ReadWrite with `CallbackEffect::NONE` (`:33-45`).
- `package_.rs:590-591`: `vsatisfies` → `WorldEffectDescriptor::EMPTY`.

### Oracle: confirmed

**Tcl 8.6.18 `generic/tclPkg.c`:**

- `PKG_PROVIDE` (`:1072-1091`): query form is `Tcl_FindHashEntry`; setter is `CheckVersionAndConvert` then `Tcl_PkgProvideEx` (`:136-180`). `FindPackage` only creates the hash entry (`:1326`).
- `PKG_VSATISFIES` (`:1272-1295`): `CheckVersionAndConvert`, `CheckAllRequirements`, `SomeRequirementSatisfied`.

**Tcl 9.0.4:** the same at `:1300-1319`, `:154-196`, `:1504-1528`, `:1562-1582`.

**Script execution:** every `Tcl_NRAddCallback` in the file is on the `require` / `SelectPackage` path. 8.4.20, 8.5.19 and 9.1.0 have the same shape for both cases.

**Runtime probe:** `package unknown` and an execution trace on `package` show no unknown-handler call for `provide` / `vsatisfies` (8.6 and 9.0). Errors only set the result and errorCode, the same as every other `EMPTY`-effect builtin.

### Consumers

| Consumer | Effect of R |
|---|---|
| `requires_world_barrier()` users: `gvn.rs:488,719`; `const_subst.rs:171`; `effect_ssa.rs:129`; `variable_bindings.rs:1331,4172`; `backend_registry.rs:1089`; `dispatch_proof.rs:1633`; `native_lowering/lower.rs:2386`; `logical_formals.rs:435`; `optimiser/method_barrier.rs:587` | Both calls stop being world barriers, which is the intent. `provide` keeps a PackageState RW access, so GVN still refuses it (`gvn.rs:719`). `vsatisfies` (pure, `EMPTY`) becomes GVN/CSE-able, which is fine: it is deterministic for the same argv. |
| Source advice (`body_effects.rs:223-249`) | PackageState is neither a trace nor a CommandBindings domain, so no uncertainty and no widen. |
| `unit_scope::scan_unit_linkage` | Trait-based, scanning Call and Barrier. Unaffected. |
| `trusted_package_transfer` | Declines `package provide tcltest 2.5;` through provision-present state, not through effects (`trusted_package_transfer.rs:72-77`). |

### Drift gates

- `xtask callback-inventory` reads argument roles and option values, not world effects (`xtask/src/callback_inventory.rs`). No generated file changes.
- `pack-goldens` covers shipped `.tclspec` only.
- `make codegen` artefacts contain no world-effect projection for these commands.
- SpecTcl parity: `world_effects` is an existing field in the loader and studio, so this is data only and needs no `GAPS` entry.
- The registry rule is respected: the knowledge is added as `CommandSpec` data and consumers stay generic.

**Note:** the Jim `provide` in `JIM_SUBCOMMANDS` (`package_.rs:632`) stays conservative. That is acceptable.

---

## B — `Condition::holds` → Inert via `tcl_dialect::validate_requirement_for`

**Patch site:** `reachability.rs:189-206`.

### Correct and fail-closed

- `validate_requirement_for(req, target)` (`tcl-dialect/src/version.rs:1030-1046`) is the shared per-release grammar owner. An `Err` returns `Ternary::Inert`, which becomes `Availability::Conditional` and is never `Available` or `Unavailable`.
- The check runs after the subject check and before `satisfies_any_ternary`, whose "malformed is No" behaviour would otherwise produce a false `Available`.
- **Oracle:** `{{8.6}}` raises on 8.6 and 9.0. `9-` raises on 8.4.

### Finding B-1 (low; follow-up, pre-existing)

The 8.4 *arity* failure has the same cause but is not covered.

- `package vsatisfies [package provide Tcl] 8.4 8.5` raises `wrong # args` under tclsh8.4.20, while the survey evaluates it as `Yes`.
- **Fix:** make it Inert when `!target.has_package_requirements() && requirements().len() != 1`, preferably derived from the registry's per-release form or arity (`VSATISFIES_FORMS`, `Arity::exact(2)`) rather than hard-coded.

### Dead code

- `Condition::TclSatisfies` had no constructor outside `cfg(test)` `Guard::condition`.
- `Walker::guard` maps `Guard::TclSatisfies` to `Unknown` first (`original.rs:620-625`).
- There are no workspace consumers: `grep TclSatisfies` finds only `reachability.rs`, `original.rs` and `tests.rs:497`.
- Mapping the `cfg(test)` arm to `Undecidable` is correct.
- The leftover test-only lexical helpers are correctly called out as blocked on a user exception.

---

## Tests likely to flip (not executed)

| Area | Selector or location | Driven by |
|---|---|---|
| Expected to pass | The NOTES list (T046, T174, T047, T178, T045, compiler 002/006/011) | — |
| Expected to pass, outside capture | `any_registry_terminator_ends_the_index`, `an_elseif_chain_selects_one_arm_per_release`, `the_real_tcllib_try_index_resolves_per_release` | C+R+B |
| Re-run, risk of change | `a_conditional_provide_does_not_make_a_document_the_provider` (`workspace_index.rs:11643`) | C+R |
| Re-run, risk of change | `package_provider()` users | D |
| Re-run, risk of change | `attested_provider_transfer_declines_unknown_revoked_and_shadowed_preludes` (`trusted_package_transfer.rs:259`) | R |
| Re-run, risk of change | `reachability/tests.rs:187` `an_unmodelled_guard_leaves_the_declaration_conditional` (the `file exists` arm now has no marker; it still relies on `Guard::Unknown`) | C |
| Re-run, risk of change | Optimiser / GVN / effect-SSA / variable-binding tests across `package provide` / `vsatisfies` | R |
| Re-run, risk of change | Analyser `param_traits` and list-quoted body tests | A |
| Re-run, risk of change | Logical navigation and workspace definition tests | D |

## 180 s nested-wrapper path

`parse_pkg_index_survives_deeply_nested_source_wrapper` (`package_resolver/tests.rs:176`) is untouched.

- **A** adds at most one O(1) `word_closer_offset_at` on a miss, and the registration `continue`s before `walk_body` (`original.rs:309-325`).
- **C** is not exercised: `ifneeded` has no `Expr` role.
- **D** is not exercised: nested `[list …]` substitutions are walked in place by `BodyWalk::substitutions` (`body_effects.rs:1124-1139`), with no branch merge.
- **R** has no effect on `list`.
