# class-facts: pause handoff, 18 November 2026

Area: `tcl-compiler` class/object/method facts (class factories, class definition scripts, method bodies, named instances, class cards, lenient `tcl` display facts) and, per the scrub table, the `lowering` and `registry_invocation` families. The registry is the source of truth: no command names in the compiler.

## (a) Landed (on `origin/work`)

17 November, evening (test scrub repairs, one cause per commit):
- `63d27a29`, `bbdb7e39`, `ea72cb7f`, `fe1f99f8`, `c8509d32`, `a2da410c`, `d5817ceb`, `4e28c829`: fixture and expectation repairs. Each was re-read against tclsh; the ledger `note` column names the commit per row.
- `9eccd049`: a TclOO creation makes its missing qualifier namespaces (`original_created_definition_key` in `command_binding/original_command_table/command_keys.rs`).

18 November:
- `f6d942f3`: **switch lowering regression root**. In `tcl-registry/src/case_bodies.rs`, `subject_selection_is_unknown` walked the options with no effect families. That dropped `-exact`/`-glob`/`-regexp`, so `switch -mode -- $x {…}` lowered to a generic Call and withdrew the command world for everything after it.
- `940dcb50`: a user metaclass that adds nothing creates as its stock metaclass (`stock_metaclass_creation_target`/`stock_metaclass_of` in `deferred_method/class_definitions.rs`, hooked into both dispatch loops in `source_walk/command_dispatch.rs`). The new receipt field `custom_metaclass` is checked by `class_definition_dependencies_hold` and refused by the default-manufacture gate (`plain_default_manufacture_class`).
- `9598eda5`: analyser-binding's review conditions on `940dcb50`. Only the named creation is stamped; there are override negatives. The review is **approved** by analyser-binding once these landed.
- `c0daade7`: a class creation after a proved error declares no class. This is ruling (7). `load_creation_proved_unreached` in `analyser/handlers/classes/class_definitions.rs` is used by `handle_oo_class_source` and `provider_definition.rs`.
- `35f97f73`: current-state class-facts section.
- Ledger commits: `3e00a3e1`, `d3607e6a`, `3e95aff5`, `515ad931`, `4d416647`, `48d44467`, `fa31ca40`, `9c26f0a2`, `c24555b1`, and this handoff.

## (b) In flight

1. **Unmarked switch subject on 8.5+.** Patch: `patches/class-facts-unmarked-switch-subject.patch`, a `git diff` against `9598eda5`. Apply it with `git apply`.
   - What it does: in `case_bodies.rs`, a dynamic subject inside the reserved trailing words (`objc - 2`) counts as known when the dialect's `tcl_version >= 8.5`. 8.4 and unpinned releases stay unknown. tclsh 8.4.20 raises `bad option` for `switch -exact $x {…}` with x=-glob; 8.5.19–9.1.0 pick the -glob arm.
   - It also extends `a_mode_option_before_the_marker_keeps_the_subject_selection_known`.
   - **Remaining:** two existing registry tests assert the old marker-only rule on tcl8.6. Both need a ruling (owner unknown, probably irules-registry-tooling or the registry owner) before their expectations change per tclsh:
     - `case_bodies::tests::selected_end_marker_uses_declared_effect_and_skips_option_values`
     - `resolved_invocation::tests::argument_roles::original_logical_case_role_projection_keeps_selected_options_and_body_residuals`, its row `[-exact, Dynamic, body]`
   - Then the full measurement.
   - values' `var_escape_cfg::ir_global_in_switch_arm_escapes` (their row `switch-name-roles-residual`) waits on this. Ping values with the SHA.
2. **Parked.** Patch: `patches/class-facts-parked-lenient-recipe-and-bodiless-receipt.patch` (`git format-patch` of `cf-parked`, based on a 17 Nov tree; expect conflicts, so use `git am -3`). It holds two commits:
   - a command substituted into a root head keeps its lenient navigation recipe;
   - a class created without a definition script keeps its receipt.
   Each regressed one test when last measured: `project_class_cards_require_selected_constructor_shape_and_compatible_inputs` for the first; its own test for the second. They are not ready.
3. **Rejected.** `patches/class-facts-rejected-labels-complete.patch` (`cf-labels`): project class cards answering from display completeness. Rejected because it answers past `oo::objdefine C unexport new` and an unknown command on the caller side. Kept only for reference.
4. **Probe tool.** `patches/class-facts-probe-test.patch` is the untracked `rust/tcl-compiler/tests/zz_probe.rs`. It prints `all_classes`, the diagnostics, top-level and procedure IR kinds (`TCL_PROBE`, `TCL_PROBE_DIALECTS`, `TCL_PROBE_IR`, `TCL_PROBE_LOWER`). It is a local aid; keep it in `.git/info/exclude`.
5. Superseded local branches, not saved: `cf-local`, `cf-merge`, `cf-viadefine`, `tmp-merge`. They are drafts of the `oo::define` route that landed as `ea179d00`, `436e60a3`, `1524a0e6`..`7bedbf45`.

## (c) Queue, in order

1. Unmarked switch subject: item (b)1. It is small, other sessions wait on it, and it is the same rule diagnostics' I231 unpinned-release row needs (keep unknown when no release is pinned).
2. **Analyser half of the user-metaclass factory.** A metaclass made by a user metaclass (`::T::Mother create ::T::D::class {superclass ::T::Mother}`) gets no factory. Its `superclass` relation has no `source_provider`, because the source-transition-advice walk (`command_binding/source_transition_advice/invocation_advice.rs`, `source_relation_provider.rs`) has no registry schema for a user-class head. It needs the same stock-creation selection there.
   - Blocks 10 `deferred_creations` rows (one is the exposed `a_later_define_replaces_the_replayed_creation_superclass`) and 6 `load_time_paths` rows, plus server/e2e metaclass chains.
   - Probe fixtures: `meta1`/`meta2`/`meta2b`/`meta3`/`pp` in (g).
3. **Qualifier namespace on the stock route.** `::T::Mother create ::T::D::class {…}` with `::T::D` absent: the slot installs, but `record_class_definition`'s lookup of the created name fails (trace: `ORIGINAL_OO_REGISTRATION offset=78 installed=false`). With `namespace eval ::T::D {}` first, it works.
4. Lowering items:
   - `try` whose body is `return -level -1 payload` lowers to Call (irules-registry-tooling, `tcl-diagram literal_invalid_return_forms_project_as_catchable_errors`).
   - procedure-target fold gap in `lower_proc_body_module_for_bytecode` (execution).
   - I231 escaped-pattern value (diagnostics).
   - `catch` under lenient `tcl` giving "malformed catch" (values).
5. registry_invocation: body/role advice drops every role when a role-bearing word is substituted or expanded (`if {1} $body`, `uplevel $lvl $script`, `eval {*}$p`). It is the diagnostics trio `param_traits` (`uplevel_records_eval_on_its_script_arg`, `deep_pass_skips_dynamic_body_args`, `trait_inference_is_dialect_aware_via_expand_syntax`); the contract (`proc-arg-traits.md`) wants them as may-traits.
6. Lenient `tcl` / Logical record (lsp-features waits on all of these; route into the Logical record per the 17 Nov ruling, never Native):
   - workspace classes reaching the walk;
   - cross-file class shadowing (`tn_an_inner_namespaced_class_shadows_the_global_one`);
   - class declarations dropped after an unknown command inside `namespace eval`;
   - list-built `[list [self] tick]` callback receivers (ten core `callback_targets` rows).
7. Tcl 9 `method foo` + `classmethod foo` instance-slot order, per class and name, last writer wins, including later `oo::define` (lsp-features: `method_and_classmethod_qnames_never_collide_on_same_name`, server `rename_disambiguates_method_and_classmethod_of_the_same_name`).
8. `my <property>` recorded as a property reference. tclsh 9.0.4/9.1.0 dispatch only `<ReadProp-name>`/`<WriteProp-name>` and `configure -name` (lsp-features).
9. Entry-independent binding, self-dispatch arm, after analyser-binding lands the query. Then uncalled receiver method bodies, lenient display facts, and `Foo new` frame contents (values).

Order reason: first unblock other sessions with small, understood fixes. Then the metaclass work, which unblocks the largest group of my own rows. The Logical-record items share one design and go together.

## (d) Waiting

- **On others:**
  - registry owner or coordinator: a ruling on the two marker-only registry tests in (b)1;
  - analyser-binding: the entry-independent binding query.
- **Others waiting on me:**
  - values: switch-name-roles-residual;
  - lsp-features: the items in (c)6, 7 and 8;
  - diagnostics: the param-traits trio and the I231 rows;
  - irules-registry-tooling: `try` lowering;
  - execution: the procedure-target fold.
- execution took codegen layer 2 of the switch regression (entry-less native switch emission) after `f6d942f3`.
- Reviews: none outstanding. analyser-binding approved `940dcb50` + `9598eda5`.

## (e) Rulings not yet carried out

- Ruling (7): add the `struct::graph g` rows, with and without a provider, plus the tclsh witness. The class side is carried out in `c0daade7`; the object-handle-type rows are not written yet.
- 17 Nov: workspace classes under `tcl` go into the Logical record (queue item 6).
- Ledger: the oracle-required newly-failing count. Measurements since the rule all ran with the required oracles, but no clean `origin/work` baseline under them exists yet, so the count is not separated.

## (f) Last measured counts

- At `a01d22ec` (= `9598eda5` before its push rebase), on the `origin/work` of about 10:30 UTC:
  - `tcl-compiler`: 1,890 failing of 13,176;
  - `tcl-lsp-core` groups: 164 failing.
- Lists: scratchpad `rev.fail`/`core-rev.fail` (not durable).
- Commands, from the repo root, after `source scripts/dev/agent-build-env.sh`, with `TCL_LSP_JIMSH=$PWD/tmp/jimtcl-oracle/jimsh TCL_LSP_REQUIRE_ALL_TCL_ORACLES=1 TCL_LSP_REQUIRE_JIM_ORACLE=1 TCL_LSP_REQUIRE_TCLSH=1 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=2 CARGO_PROFILE_TEST_OPT_LEVEL=1`:
  - `cargo nextest run -p tcl-compiler --no-fail-fast --status-level fail --final-status-level none --failure-output never`
  - `cargo nextest run -p tcl-lsp-core --lib --no-fail-fast -E 'test(/^(rename|references|definition|workspace_index|call_hierarchy|semantic_tokens|document_symbols)::/)' …` (same flags)
- That was not a clean-head baseline; take one before quoting fixed counts.

## (g) What a successor would get wrong

- **Baselines.** Compare only against a run at a clean `origin/work` head under the same oracle settings. Every push rebase moves the base. Attribute any "new" failure by re-running just those tests with your change reverted (`git checkout HEAD~1 -- rust/tcl-compiler/src`, run, then `git checkout HEAD -- …`).
- **Shallow clone.** If `git rebase origin/work` floods add/add conflicts, abort and `git fetch --deepen=200 origin work` first.
- **Disk.** `CARGO_INCREMENTAL=0` always. Prune `target/debug/build/tcl-compiler/*` (keep the newest ten) and `rm -rf /tmp/tcl-spectcl-cache-test-*`.
- **`cargo fmt -p tcl-compiler`** reformats other sessions' files. Format only your own files (`rustfmt --edition 2024 <files>`), and never `git commit -a` after a crate-wide fmt.
- **Display vs proof.** `all_classes` and `original_class_declarations` are two separate recording paths for one creation; change both, or `load_creation_proved_unreached`'s sibling. Display consumers may fall back to written words; proof consumers stay strict.
- **TclOO dispatch for `X create NAME`.** The method is found through X's metaclass, then that metaclass's superclasses. The constructor that runs comes from X's superclass chain. A class mixin or filter on Meta changes `N create`, not `Meta create` (tclsh 8.6.18/9.0.4).
- **Fixtures.** `$argv` reads as unset at top level, which proves an error, so everything after it is the recovered tail. Bind probe subjects with `[clock seconds]`.
- **Lowering probes.** `lower_to_ir` is the isolated entry. The bytecode compile service (`vm_compiled::procedure`) is the product route. Check both before calling a lowering fault a product regression.
- **Probe fixtures for the metaclass work:**
  - `meta1`: `namespace eval ::T {}; oo::class create ::T::M {superclass oo::class}; ::T::M create ::T::E {method m {} {}}`. Works.
  - `meta2`: Mother/`D::class {superclass ::T::Mother}`/W. Analyser factory missing.
  - `meta2b`: the same with `namespace eval ::T::D {}` first.
  - `pp`/`COMPUTED_METACLASS` in `tests/it_4/analyser/class_factories.rs`: the real fixture.
