# Luna-25: static review of Sol-22 `db921-merged-v1.patch`

**Scope.** Repo `/home/user/tcl-lsp` at HEAD `0c3d7732e`, branch `work`, with a clean working tree. I ran no cargo, made no writes to the repo and created no worktrees.

**Apply check.**
- `git apply --check --cached` under a temporary `GIT_INDEX_FILE`: **OK** against HEAD.
- `p/a` is byte-identical to HEAD for all 36 files.
- `diff -ruN p/a p/b` reproduces the patch exactly.

**Verdict: ADMIT-TO-BUILD.** The patch is fit to compile and measure, with the pre-land conditions in §6.

I found no site where the patch silently changes an analysis result, and no count gate that newly admits. The pairing is the existing `covers_installed_analysis_source` relation, and the carried Installed lowering keeps every identity it had. Two concrete defects must be fixed before landing: D1, fail-closed hardening of `pairs`, and D2, a quadratic cost on every standalone build. Neither changes behaviour on the named test fixtures, so measuring v1 is valid evidence.

## 1. Soundness of the merge

**What `pairs` is built from.** The only source of pairs is `ReadonlyAuthoredCoverage::relate` in `Pairs` mode. Per lowering, `classify_lowering` runs the original `capture` checks unchanged:
- ownership, `function_for_body` pointer equality, metadata input and `body_id`;
- authored: not installed, rebased original CFG, original summary, `matches_procedure`;
- Installed: lexer config, `installed_procedure_cfg_for_body`, installed summary, a unique row by allocation, `matches_installed_row`, and `unique_authored_source`.

`unique_authored_source` uses `covers_installed_analysis_source` (`registry_invocation/authored_source_bodies.rs:198-221`). That function requires:
- First incarnation;
- equal allocation site;
- equal `namespace_key` and parameters;
- origin equal to the card's site source;
- a Contiguous mapping at the card's body start;
- byte-equal executed text.

So a pair can never join different commands (site, incarnation, origin, namespace, formals or executed source).

The new site pre-filter (`card.site() == &installed.allocation.site`) is a necessary condition of `covers` (`:207`). It cannot widen the relation or narrow what `covers` admits.

Two further checks hold the pairing one-to-one:
- Injectivity: two admitted Installed lowerings that map to one authored source put it in `shared` and drop it.
- The authored target must itself classify as `Authored`, and must be in `body_units` rather than carried.

**Complete mode (`capture`) matches the old behaviour in the uncarried layout.**
- `Other` or a refusal gives `None`.
- A shared source gives `None`.
- The `retain` is a no-op in this mode.
- In the carried layout it additionally refuses a carrier that is not the re-derived authored source; Test 3's new case exercises this.

**Identity retained by the carried lowering.** `carry` moves the `Procedure` and `FunctionUnit` unchanged under their own key. All of these are untouched:
- `ProcedureId` and its ownership by `procedure_ids`;
- `installed_procedure_body_units[id]`, the allocation tag;
- `source_body_cfgs[id]`, the full-config CFG;
- `RetainedSourceModuleBindings.installed_procedures[id]`, which was computed at lowering time and is now read through `body_lowering`;
- `body_kind == Installed`, `executed_source`, `namespace_context` and `function.body_id`.

The authored unit keeps `executed_source == None` and its rebased CFG. `carry` acts only when all four store entries are present.

**Every CU construction reaches the carry.** `source_body_cfgs` is private and has exactly two struct-literal sites: the `:3421` fixture and the `:4023` build. Every public `build_*` funnels into `build_with_retained_entry_and_source_syntax`. `specialise_factories` runs before the build.

**D1 (fix before landing; fail-closed).** In `Pairs` mode, `shared` is computed only over Installed lowerings that classified successfully.
- Suppose an Installed lowering I2 is refused by an integrity check before `unique_authored_source` (lexer config, CFG, summary, or a duplicate or mismatched row), but it covers the same authored A as an admitted I1.
- Then I1 is carried into A alone, which asserts a one-to-one relation the module does not show.
- A refused lowering is also not necessarily a non-cover: `CommandAllocation` includes `command` and `namespace` as well as the site, and `covers` does not compare `command`.
- On a fresh build this is unlikely, but the rule is fail-closed.
- Fix: before carrying A ← I, require that I is the only entry in `installed_procedure_body_units` whose `allocation.site == card.site()`. This is conservative: a proc defined in a loop with later incarnations simply stays uncarried, as today.
- An alternative is to evaluate `unique_authored_source` for every Installed-tagged lowering whose row exists, whether or not it was admitted, and mark the source shared.

## 2. Consumer completeness

I rescanned all `body_units` sites. I did not exclude lines that also mention another `*_body_units` field, as `sites2.py` did. The scan found 124 non-test lines, against Sol-22's 123. The extra line is `compilation_unit.rs:3992` (`let body_units = Self::build_body_units(`), which is at build time (B) and harmless.

I checked every N, U and B classification against the code:
- **Authored, native, logical or hosted card-keyed `body_units.get`** (N; carried Installed IDs are never cards):
  - `analysis_body_correspondence.rs:287/340/1056`
  - `authored_body_scope.rs:589`
  - `original_procedure_call.rs:189/226/602`
  - `native_declared_body.rs:535`
  - `ssa.rs:2115`
  - `interprocedural.rs:2604`
  - `body_analysis_capsule.rs:409/422`
  - `source_graph.rs:455/484/503`
- **`ir.rs:3086-3128` `original_source_procedure_bodies`, and `:3448` `executable_script_roots`**: both exclude installed bodies (N).
- **`compilation_unit.rs:4540`**: filtered to `original_source_procedure_bodies`, so installed functions never received interprocedural taints at HEAD either.
- **`:4669`**: filtered by `original_procedure_for_body`, which is `None` for installed (N).
- **Emptiness gates**, all U, unchanged because a carrier implies a non-empty `body_units`:
  - `common_aot_plan.rs:946`
  - `unit_scope.rs:1749/1909` (build time)
  - `body_analysis_capsule.rs:476`
- **`interprocedural::summary_procedure_bodies`**: named bodies plus `OriginalDeclaration` original bodies only. Interprocedural summaries cannot see either layout's Installed lowering.

**Gates kept refusing (†), all four verified:**
1. `analytical_closed_projection` `source_body_inventory`, both branches.
2. `report_comparison.rs:261`. The count over lowerings at the site stays 2.
3. `original_report.rs:182`. An installed ID is not in `required`.
4. `tcl-lsp-db/src/lib.rs:3800` `every_proc_keyed`. `original_procedure_for_body(installed)` is `None`.

**Further searches, no new gate found:**
- `.len()` / `.is_empty()` / `.count()` on `body_units` and on the CU function accessors. `functions()` is the top level plus procedures only. `all_body_function_units`, `body_function_units` and `analysable_methods_and_body_units` are converted at the source.
- `installed_procedure_body_units`-keyed lookups: `retained_source:181`, `object_types:1018` and `compilation_unit:4931` are converted. `installed_bodies:334` and `specialise_factories` run at lowering time.
- Maps passed by another name: none. No `body_units` map is passed to a helper.
- Mutation loops after the build: `with_retained_memory_ssa` covers procedures only. The semantic-fact loops are converted.
- `OptimiserSourceOwner::matches_unit` uses `==` plus `all_body_function_units` and `function_for_body`, all carry-aware. `top_level_view` and `original_report` clear the carried store.
- `procedure_body_for_declaration` callers resolve the ID through:
  - `function_for_body` / `fu.body_id`: `dataflow.rs:1364/1405/1547`;
  - pointer equality against a body they already hold;
  - or the converted `infer.rs:277`.
  - `tcl-lsp-core` and `tcl-diagram` never index `body_units` directly.

**Test-only observations (non-blocking):**
- The tests that index `body_units` almost all use authored or native card IDs, or are lowering-only (`lower_to_ir`), so the carry never runs for them.
- The ones that touch Installed IDs on a CU build are the coverage tests (corrected in the patch) and `interprocedural.rs:7572`, which already uses carry-aware accessors.
- `tcl-spec-studio` test `infer.rs:1095-1103` renames `procedures` and `body_units` only. If its fixture ever carried, the collision would be silently skipped for the carried lowering. Its expected facts imply that the fixture takes the named or native path, so it is unaffected today. Consider extending the rename loop to `carried_installed_lowerings` so it cannot become vacuous.

## 3. Equality, hashing, durable inventory and Explorer

- `CompilationUnit::eq` gains `carried_installed_lowerings`, so equality is stronger. `Module` derives `PartialEq`, and `CarriedInstalledLowerings` derives `PartialEq` including `unit`. Neither type has a `Hash` impl.
- The `durable_inventory!` roster and its exhaustive witness add the field, and the `ARTIFACT_COVERAGE` row "semantic" is added.
- `all_snapshots` lists the carried function as `BodyUnit`.
  - **Optional:** a distinct kind, or a carrier annotation (`unit_of`), so Explorer shows one unit with two lowerings.
- **Pre-existing, not caused by this patch:** `ARTIFACT_COVERAGE` at HEAD has no rows for:
  - `FunctionUnit::body_id`
  - `FunctionUnit::original_hosted_procedure_body`
  - `CompilationUnit::source_class_facts`
  - `CompilationUnit::source_body_cfgs`

  `every_durable_field_is_reviewed_in_the_coverage_table` should therefore already be red. Do not attribute that failure to DB921 when measuring.

## 4. The `readonly_authored_coverage` test corrections

They are the narrowest consistent with the decision.

**Unit counts.** Only the unit counts change from 4 to 2, which the decision forces. The "4" is kept as `module.body_lowerings().count() == 4`.

**Checks added, each strengthening:**
- `carried_installed_lowerings.len() == 2` on both stores;
- `unit_of(installed) == source_for(installed)`;
- `!body_units.contains_key(installed)`.

**Unchanged.** Every refusal is unchanged: duplicated owner, three config axes, foreign input, changed source, missing authored, swapped allocation, foreign `params_raw`, `body_id`, and foreign registry. Mutations are re-targeted to the store that now holds the Installed lowering.

**Added refusal.** Test 3 refuses a carrier that is re-pointed. The case is not vacuous: every precondition holds, and only the carrier check fires.

**New test, sound.** `carried_lowerings_are_exactly_the_readonly_authored_pairs` checks that `pairs` on a carried unit is `None`, that un-carrying restores `capture` and `pairs == unit_of`, and that re-carrying gives `==`.

**Suggestion.** Also assert `unit.body_lowering_functions().count() == 4` in test 1.

**The DB assertion `lib.rs:12034`** is correctly left untouched. It is measurement-gated, because of the `from_grammar` versus `for_file_grammar` config difference that Sol-22 flags.

## 5. Static compile and clippy read

I found no likely compile errors.
- `ProcedureInventory::iter`, `CarriedInstalledLowerings::lowerings`, HashMap `iter` and `&HashMap` all yield matching item tuples, so every `chain` type-checks.
- `iter_mut().chain(&mut map)` yields `(&K, &mut V)` on both sides. The loop bodies borrow the disjoint field `self.ir_module` only, and never a `&self` method.
- In `retain(|_, &mut source| …)`, `ProcedureId` is `Copy`, so the pattern is valid.
- `match … Some(Other) | None if inventory == Pairs` is exhaustive.
- `&rows` coerces from `Vec` to a slice.
- In `report_comparison`, `Some(site)` is compared with `site: &CommandAllocationSite`, and the types agree.
- The test-only `ProcedureInventory::insert` is `pub(crate)` within the same crate.
- Pedantic lints: `lowerings` avoids `iter_without_into_iter`; `len` has a matching `is_empty`; the iterator-returning functions are must-use through `Iterator`.
- No new `#[allow]`, and no non-test `unwrap`.

## 6. Defects and conditions

| ID | Severity | Item |
|---|---|---|
| D1 | must fix before landing | `Pairs` shared detection ignores refused Installed lowerings (§1). Add the per-site Installed uniqueness guard. |
| D2 | must measure; fix before landing | `pairs` now runs on every standalone build and is Θ(n²) in authored cards (see below). |
| D3 | perf, minor | `Module::body_lowerings()` allocates and sorts on every call, and is called once per declaration in `procedure_body_for_declaration` (via dataflow peers and diagrams) and once per body in `report_comparison`, which is O(n² log n). Both stores are `BTreeMap`s sorted by `ProcedureId`, so return a non-allocating sorted merge instead. |
| D4 | docs, before merge | Add a sentence to `name-resolution-implementer-guide.md:3896-3901` ("It records the result in `Module::body_units`…"), noting that a covered Installed lowering is carried after the unit build. Optionally update the `shared-utility-contracts-rust.md:236` inventory row and add a GLOSSARY term. Reflow the over-long `architecture.md` line. Record the test-assertion corrections (4→2) in the active tracking documents as a decision-forced change. |
| D5 | measurement | Run the suites the decision lists: coverage (the three changed tests plus the new one), `interprocedural`, `object_types`, `source_graph`, `ssa`, the DB test `lib.rs:12034`, Explorer coverage (noting the pre-existing red), `tcl-diagram` and `tcl-lsp-core` graphs, `tcl-spec-studio` `infer`, and `cargo xtask owner-resolution` for the new `pairs` row. Then `make rust-check`. |

**D2 in detail.** The Authored classification calls `CompilationUnit::original_procedure_cfg_for_body`. That method runs `original_source_procedure_bodies().any(..)`, which evaluates `card.matches_procedure` for every earlier card, so the cost per authored ID is O(n). `matches_procedure` re-derives the header and compares the whole `Script`.
- This contradicts the DESIGN §3 claim that the per-build O(n²) is avoided.
- Fix: compute the original-body ID set once in `relate`, and gate the `source_body_cfgs` lookup on it through a `pub(crate)` variant. Results stay identical.
- Measure on a large standalone fixture, for example 2,000 procs, and with `benches/value_transfers`.

**Out of scope, noted.** Native-declaration and Installed pairs stay two units. The decision names the readonly authored lowering, so this is consistent, but a future DB-shaped test on the native path would see two units.
