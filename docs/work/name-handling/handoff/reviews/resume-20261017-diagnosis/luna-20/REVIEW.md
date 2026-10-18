# Luna-20 review of the Sol-18 patches

**Scope.** Repo `/home/user/tcl-lsp` at HEAD `9cc6b4089`. The review was static: I ran no cargo. Probes ran on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Probe scripts and outputs are in `luna-20/probes/` (`fi.*`, `c221.*`, `vs.*` and `idx/`).

**Apply.** Each of the four patches passes `git apply --check`, both against the working tree and with `--cached`. All four concatenated also apply. They touch different files, and I found no semantic interaction between them.

The working tree carries another agent's uncommitted edits. One of them is a debug `eprintln!("CONTROL …")` in `source_structure/package_target_survey.rs`, which must not ship. The vsatisfies patch still applies over it.

| Patch | Verdict |
|---|---|
| compiler221.patch | **ADMIT-WITH-CONDITIONS** |
| db921-v2.patch | **HOLD** |
| vsatisfies-84.patch | **ADMIT-WITH-CONDITIONS** |
| forced-import-assertion.patch | **ADMIT-WITH-CONDITIONS** |

## 1. compiler221.patch (`source_class.rs:1081`: `is_none` → `is_some`)

**The oracle holds.** I re-ran the fixture on 8.6, 9.0 and 9.1. Each run returns code 0 and creates `::outer::inner::Widget`, a class whose methods are `m`. Nothing is removed and the namespace is restored. There is one fixed world.

The sibling test asserts `Some` for the identical input, dialect and constructor, and it passes at baseline.

**Later assertions that have never run:**
- The class name, `c_source_body` and `logical_source_input` assertions already pass in the `Some` test.
- `methods` contains `m`, which matches tclsh.
- `ConditionalCBodyApplicability` is pushed together with `c_source_body` (`source_class.rs:741-755`).
- `ConditionalCNamespaceApplicability` is pushed under the same predicate that gates `c_source_body` (`source_transition_advice.rs:2706`).
- `RegisteredFactoryApplicability` is pushed unconditionally (`:758-760`).

All of these are likely to hold. The refusal controls match tclsh. `namespace eval $unknown` raises. `proc ::oo::class` followed by the create call returns code 0 and creates no Widget. The Native-baseline refusal is a policy rule (no source-to-Native grant) and does not contradict tclsh. I found no assertion that contradicts tclsh.

**Conditions:**
1. Update `docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md`.
   - Line 140 describes this control as "completed command worlds remain unavailable".
   - Line 18 ends "…supply no completed command world". Reword it to say that the class cards do not supply the world, and that the independently completed world is retained.
2. Execute the test. If a later assertion fails, that is a separate defect. Do not revert the assertion.

## 2. db921-v2.patch (`installed_bodies.rs`): HOLD

**What is sound.**
- It never merges distinct commands. A merge needs the same site, First incarnation, origin, namespace key and formals, plus an equal rebuilt contiguous executed source.
- A mismatch keeps both units.
- The `already_named` refactor is behaviour-identical.

**Why I am holding it.**

1. **Three original assertions turn red.** `body_analysis_capsule/readonly_authored_coverage.rs` builds `proc kept …; proc other …` with `build_with_options` under tcl8.6 standalone, the same lowering as the DB test.
   - The fixture asserts `installed_procedure_body_units.len()==2` and `body_units.len()==4`.
   - It also asserts that each installed unit is covered by a distinct authored unit, through `covers_installed_analysis_source`.
   - The covers criteria are a superset of the patch's equality, so `already_authored` fires for both procs and the installed count drops to 0.
   - `…preserves_both_actual_analysis_inventories` fails its `assert_eq`. `…keeps_installed_function_input_and_unique_store` and `…refuses_missing_foreign_and_mismatched_owners` panic on `.next().unwrap()`.

   This is the same kind of same-input contradiction as Compiler221, against the DB test's `procedures+body_units==1`. The patch neither resolves it nor flags it.
2. **The Installed identity is lost, not merged.**
   - The surviving authored unit has `executed_source == None`.
   - Several consumers pair an implementation allocation with an analysis unit through `executed_source`: `object_types::procedure_implementation_units` (`object_types.rs:984-1020`) and the declaration-body selection in `source_graph` (`source_graph.rs:172-189`).
   - Others key on `installed_procedure_body_units`: `installed_procedure_cfg_for_body` and `RetainedSourceModuleBindings.installed_procedures` (`retained_source.rs:181`).
   - After the patch, none of these reaches the proc's body. `already_named` is sound only because the kept named procedure carries `executed_source`.
   - The 11:55Z ruling makes the two bodies one identity. It does not allow dropping the Installed links, and `current-state.md:44` forbids erasing Source SSA identity.
3. **It re-derives an existing correspondence.** `OriginalAuthoredProcedureBody::covers_installed_analysis_source` already owns this check. With the patch, `ReadonlyAuthoredCoverage.installed_sources` can never be non-empty, so that module becomes dead.

**What would make it admissible:**
- Merge into the authored unit while keeping the allocation and executed source reachable, or let the pairing consumers accept the covers relation.
- Resolve the `readonly_authored_coverage` assertions under the ruling, as a documented narrow correction or by retiring the module.
- Use `covers_installed_analysis_source` as the merge predicate.
- Measure the DB test together with the coverage, interprocedural, `object_types` and `source_graph` tests.

## 3. vsatisfies-84.patch: ADMIT-WITH-CONDITIONS

**C behaviour.**
- In `tclPkg.c` 8.4.20 (non-TIP268), `objc != 4` raises `wrong # args`.
- `package.tcl:503-510` (`tclPkgUnknown`) catches the error, logs "error reading package index file", and abandons the rest of that index. Declarations made before the error stay registered.

**My re-probe** used the exact test fixtures plus a "before" control:

| Release | Loaded |
|---|---|
| 8.4 | nothing after the guard |
| 8.5 / 8.6 | `b1pkg` and `thenpkg` |
| 9.0 / 9.1 | `elsepkg` only |

This matches every new test row. Returning `No` regardless of `expect` correctly makes both arms, and the negated after-chain gate, Unavailable.

The release data comes only from Registry `VSATISFIES_FORMS`, iterated over `TclVersion::ALL`. No consumer hardcodes a version or name. No existing assertion changed: one doc comment was corrected and the receipt test only gains assertions. Reading the code, the APIs and types it uses exist (`SubCommandForm = CommandForm`, `TclVersion::ALL` is an array, and so on).

**Conditions:**
1. **Do not treat an unknown as a rejection.**
   - `arity_accepts_under` returns `None` for an unknown count, and `authoring_query()` can be `None`.
   - The survey's `== Some(true)` turns both into "rejected", so `Condition::holds` returns a definite `No`.
   - Keep the result tri-state, for example by recording only `Some(false)` as a rejection, so that an unknown count stays Inert.
2. **Align the new registry helper with its owner.** Recount per form with `authored_source_count_for_arity(form.arity)`, which that function's doc requires. Decline for `arity_windows`, `STRUCTURALLY_CHECKED_ARITY` and unresolved subcommands, as `authored_source_arity` does. Alternatively, narrow the helper and document its scope.
3. **Compile and run the touched tests.** That means `reachability::tests`, the `package_` registry tests and the `package_target_survey` tests.

**Residuals.** None of these is made worse by the patch.
- A raise in a non-terminating chain does not abandon the declarations after it on 8.4.
- The requirement-grammar raise is still Inert.
- A subject other than the interpreter release returns Inert before the arity check.
- 8.4 builds with `TCL_TIP268` accept multiple requirements. The stock oracle does not.

## 4. forced-import-assertion.patch: ADMIT-WITH-CONDITIONS

**Independent re-run on 8.4.20 through 9.1.0.** I ran it in a child interpreter and as a top-level script, with and without `auto_import`. Every version gives `invalid command name "p"` with `::dst` empty.
- After `-force`, `namespace origin` is `::src::p` and the call returns SRC.
- After the forget, `::dst` is empty.
- In the control, the unforced import raises `already exists` and the call returns LOCAL.

The fixture text is byte-identical to the test's. `is_none()` is the exact oracle value. The positive control `a_forced_import_shadows_the_local_command` (`definition.rs:6350`) keeps the suite from being vacuous.

**Condition:** execute the test before landing.
- A static trace suggests it may be red at HEAD. After the forget, `live_import_at` drops the forced edge, so `forced_import_shadows` becomes false, and `proc_visible_from_namespace` can return the deleted local `::dst::p`.
- An earlier baseline recorded the old assertion as FAILED (`sol-5/dbg/base_def.txt`), so what the test returns today is unknown.
- If it is red, the implementation fix must ship in the same change: a forced import deletes the local declaration, and a later forget does not resurrect it. Do not revert the assertion.

**Optional:** rename the test at final cleanup.
