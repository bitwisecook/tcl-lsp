# Hand-over: dataflow (#2408, #1796)

Written 17 November 2026 by the outgoing session, paused since 16 November, as a message to the orchestrator and filed here unedited. The successor is the `dataflow` issue session.

The paths and commits below predate the file split of 16 November: see *Starting* in `../README.md` before applying anything.

```text
HANDOFF — group `issues/dataflow` (bitwisecook/tcl-lsp). Session https://claude.ai/code/session_017u5g3KFawEtRopEfY5omDk. Everything is pushed; the tree is clean.

## 1. Issues
- **#2246: closed** (comment 6899373753). Fixed in 2485cc4d on `work`. Nothing remains.
- **#2408: open.** Before it can close:
  (a) Land 4a61cc0a on `work`. It holds the soundness fix plus the gate (see §2).
  (b) Release summaries currently exist only for procedures the module never calls. So `c`→`bump` composition, the 8 `interprocedural::transfer::tests`, and the coordinator's close criterion (transfer, frame and `upvar_info` tests passing again) all still need step (c).
  (c) Give every procedure a declared-frame body unit in release builds (see §6).
  (d) Option B (callee summaries feeding the caller's walk; maintainer decision 6900373209).
  (e) Message the iRules session (session_017u5g3KRpbHebfAxzbMbnAe) with the commit, so it can add #1796's `uplevel 3` witness.
- **#1796:** moved to the iRules session. I only posted comment 6899380975. It is blocked on the same step (c).

## 2. Branches and commits
**On `work`:**
- 2485cc4d — #2246.
- 8e7f85e1 — step 1a: `SourceSummaryContext` admits a release unit's `AuthoredSimulation`, with an authored realm, purpose-checked metadata, operation, `procedure_call` via `original_authored_procedure_calls_for_module`, and `transitions()`.
- 90b72bb7 — step 1b: authored variable operands, the alias local and source place, and `authored_outer_writes`. Release summaries flow. **It has a soundness gap:** a called procedure, walked in its callers' joined frame, can be summarised as error-only (seen with `bump` called from an uncalled `c`).

**Only on `issues/dataflow`:** 4a61cc0a ("Keep the summaries a rebound builtin never reaches"). It contains:
- `walked_in_declared_frame`, the soundness guard;
- the #2408 gate: `nameable_rebound_builtins`, `spells_a_rebound_builtin`, `may_name_a_summary` narrowed;
- removal of `rebinds_builtins`;
- an updated `docs/design/compiler/precision-limitations.md`.

**Land it first:** rebase onto `origin/work`, run clippy, push `HEAD:work`.

## 3. Verification
- **Environment:**
  `export CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 RUST_BACKTRACE=0 CARGO_TARGET_DIR=/home/user/tcl-lsp/target`
- **Build:** `cargo test -p tcl-compiler --lib --no-run`. The binary lands at `target/debug/build/tcl-compiler/<hash>/out/tcl_compiler-<hash>` (here 0350b16347ca8842). Copy it somewhere before running long suites, because rebuilds overwrite it.
- **Full suite:** from `rust/tcl-compiler`: `<bin> --skip depth_guard::tests::the_source_walk_cap_fits_its_stack_budget > log; grep -E '^test .* FAILED$' log | sort -u > fail`. About 25 minutes.
- **Baseline:** take it from the same binary built on unmodified `origin/work`. Compare with `comm -13 base new` for new failures and `comm -23` for fixed ones.
- **Results:** baseline 2 at 41990a3d had 1,773 failures. 4a61cc0a: 0 new, 46 fixed (1,727 failing). Clippy and the gates were not run on 4a61cc0a.
- **Gates:** `cargo clippy -p tcl-compiler --all-targets -- -D warnings -A clippy::assert_is_empty`, then `cargo xtask registry-axes`, `cargo xtask value-transfers --check`, `cargo xtask owner-resolution`, `cargo xtask callback-inventory --check`. Also `rustfmt --edition 2024` on changed files. All were green on 90b72bb7.
- **tcl-registry:** `cargo test -p tcl-registry --lib --tests --no-fail-fast` has 9 baseline failures. It was 0 new for #2246.
- **Witnesses** (8.4–9.1): `a_release_unit_has_an_authored_summary_context`, `release_units_summarise_their_procedures`, `a_shadowed_builtin_keeps_the_summaries_that_never_reach_it`, `w210_dict_with_reads_the_value_after_keyed_updates`.

## 4. Design decisions
- **Maintainer, 6900373209:** option B, with module-closed facts as floor A, and trace possibility taken from `ModuleTraceFacts`. My options and the analysis are in 6899807198. The scope and plan are in 6900681456 and 6902047236.
- **Transfer tests use the logical fixture.** `logical_completion_unit` is a projected Jim 0.79 profile with no `execution_name_policy`, so `original_command_head_input` is `None` for generic heads. It can never resolve `return` or calls. On `rust` these tests used `build_for_dialect`. Moving them is a category-2 fixture fix once composition works.
- **Release frames:** `SourceOriginalVariableFrame::from_authored_procedure` takes the body's `executed_source`, the namespace, and identity `source_activation_name(site.source, qualified_name, site.offset)`, which equals the walk's declaration frame. `same_frame_commands` skips `UpFrame` and namespace-changing `Block`.
- **Linked params are `ANY_OUTCOME` in release** (`reads_linked_locals`). Release SSA writes through an `upvar` alias to its target, so `incr v` has no definition of `v`. Reading it would have reported "Preserve", which is unsound.
- **#2246:** `dict` keyed updates got `VARIABLE_READ_MODIFY_WRITE` world effects. `keyed_update_store.rs` uses `value_transfer::literal_keyed_update` and keeps only `Constructed(Dict)`; append, incr and lappend have their own owners. The receiver uses `whole_array=false`.
- **The contract test** `original_transfer_shadow_and_command_observers_do_not_borrow_catalogue_closure` now asserts that a release unit has a context and a Standalone unit has none. The doc `interprocedural-original-transfer-summary-source-context.md` is updated to match.

## 5. Dead ends
- **No culprit to bisect:** summaries had been failing since `work`'s root e5b4d0c2f.
- **Do not use a worktree on the shared target dir:** it poisoned `tcl-core-types` and needed `cargo clean -p tcl-core-types`.
- **`original_authored_scalar_body_scope` is `None` on release-build body tokens.**
- **Called procedures don't use their own frame:** their `binding.variable_frame` is `Unknown` (a join), or the entered caller activation when the caller runs. `original_native_declaration_body_units` exists only for uncalled procedures.
- **`set` resolves `state_transitions` as `UnknownInvocation`;** use `schema.authored_source_effects_and_state_transitions`.

## 6. Next steps and pitfalls
1. Land 4a61cc0a.
2. In the build pipeline (`compilation_unit.rs` `module_procedures`, and the lowering of `original_native_declaration_body_units`), produce a declared-frame lowered body (CFG, SSA, bindings) for every procedure, including called ones. Point `ModuleProcedures` (`flow_of`, `roles`, `local_calls`, `same_frame_commands`) at those bodies, then drop the `walked_in_declared_frame` refusal for them.
3. Make release SSA define the linked local at a write through a caller link, so `upvar` + `incr` gives `Bind`. Then remove the `reads_linked_locals` fallback.
4. Move the 8 transfer tests to `build_for_dialect` per release, restore the `c`/`bump` gate witness, then do option B and close #2408. Ping the iRules session.

**Pitfalls:**
- The machine has 4 cores and 15 GB. Run one cargo build at a time.
- `--release` is forbidden.
- Never edit source while a build is compiling, or you get a stale or broken binary.
- About 2,500 failures across the crate are upstream (the naming burn-down).
- The burn-down sessions edit `source_context.rs`, `logical_variable_operands.rs` and `realm.rs`.
```
