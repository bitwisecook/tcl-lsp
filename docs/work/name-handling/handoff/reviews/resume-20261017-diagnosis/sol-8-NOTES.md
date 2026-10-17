# Sol-8 notes: the four integration37 tests that still fail

Static only. I did not run cargo, any test binary or gdb, and I did not modify anything under `/home/user/tcl-lsp`. The only tool I ran was `rustfmt --edition 2024 --emit stdout` on stdin copies, at nice 19.

Integration37 is now committed as HEAD `60051c3c2` (parent `f606aa20c`), and the working tree is clean. Every line number below is for HEAD. Both patches pass `git apply --check` against the current tree.

| # | Test(s) | Root cause | Confidence | Deliverable |
|---|---|---|---|---|
| 1 | `references::tests::constructor_next_chain_reference_via_{nextto_explicit,braced_nextto}_target` | A native (tcl8.6) analysis never reaches the next-chain resolver. The original-declaration route selects the `constructor` keyword as an `OriginalDeclarationRole::Special` identity, and `reference_spans` returns `Vec::new()` for `Special`. The nextto canonicalisation is not the blocker. | High | `fix-1-native-lifecycle-next-chain.patch` |
| 2 | `original_declaration::own_object_identity_tests::original_own_object_identity_keeps_uri_currency_and_wire_reselection` | The compiler's own-object method producer only accepts the script form `oo::objdefine obj {…}`. The inline form `oo::objdefine obj method …` installs no own receiver-method entry. As a result, `object p\uD800` gets no method query and no reference. | High (cause); no patch | Design below |
| 3 | `tcl-lsp-db project_token_inputs::tests::project_token_hints_withdraw_when_another_members_source_owner_is_unavailable` | Narrowed but not pinpointed. The empty `other` file is very probably fine. The main file's `OriginalProjectProcedureRoles::capture` returns None, most likely because the class header has no logical coordinate. | Medium (narrowing) | `probe-3-roles-capture.patch` (a diagnostic test, not a fix) |

---

## 1. Constructor next-chain references with `nextto`

### What actually happens
- The test helper analyses with `"tcl8.6"` (`references.rs:4849-4852`). tcl8.6 has a C native name protocol, so `has_logical_source_name_context()` is false (`analyser/input.rs:86-96`). That makes `allows_lexical_declaration_advice()` false (`analyser/types.rs:3119-3127`). The test at `source_class.rs:1004` asserts exactly this for tcl8.6 and tcl9.0.
- `references_in_program` (`references.rs:948`) runs `original_declaration::select` at `references.rs:1016`. The `select` function (`original_declaration.rs:709`) only continues when lexical advice is allowed. Here it reaches `declarations()`, which includes `for_special` rows. Their span is the lifecycle keyword (`original_declaration.rs:1643-1647`, `:372-399`). A cursor on `constructor` therefore returns `Break(Some(<Special identity>))`.
- `references.rs:1019` → `readonly_reference_report` → `reference_spans`. That function returns `Vec::new()` for `OriginalDeclarationRole::Special` (`original_declaration.rs:926`), so the result is `[]`, which matches the log (`left: 0`, message `[]`).
- Even without that `Break`, `references.rs:1037` (`if !analysis.allows_lexical_declaration_advice() { return Vec::new(); }`) would stop the call before `constructor_or_destructor_references` (`references.rs:1176`/`:1245`). So `constructor_next_chain_references` (`references.rs:2371`) is unreachable for any native analysis.
- The integration37 nextto change (`crate::definition::canonicalise_class`) is correct, and it is why `jump_to_nextto_named_class_method` went green. But that change sits behind this gate. The Sol-4 and Luna-4 prediction followed only the canonicalisation and never the routing.
- Implication, not measured: every native constructor/destructor next-chain test has the same cause, plain `next` included. That covers `constructor_next_chain_reference_from_direct_subclass`, `_skips_ancestor_with_no_own_constructor`, `destructor_next_chain_reference_from_direct_subclass` and `constructor_keyword_resolves_its_own_next_chain_even_with_a_same_named_method`. It probably also covers the native code-lens tests `constructor_lens_counts_subclass_next_chain` and `destructor_lens_counts_subclass_next_chain`: `code_lens.rs:119-138` counts native lenses with the same `readonly_reference_report`. None of these tests is in the 1085-selector capture, so there is no recorded verdict for them.

### Fix (`fix-1-native-lifecycle-next-chain.patch`)
- `original_declaration.rs:926`: `Special` now reselects its class record and special row (`class_metadata` plus `special_metadata`, both already used by `is_current`). It then returns `crate::references::original_special_next_chain_spans(...)`. `Document` still returns `Vec::new()`.
- New `references::original_special_next_chain_spans` is the native counterpart of `constructor_next_chain_references` / `destructor_next_chain_references`:
  - It returns nothing unless the selected special row is the effective (last) row of its kind and side in its class. A shadowed constructor gets no chain, as before.
  - For every other `original_class_declarations()` record, it scans that record's effective special body of the same kind and side with the existing `scan_next_dispatch_sites_with_target`.
  - It canonicalises a `nextto` target with the existing `crate::definition::canonicalise_class`.
  - It keeps a site only when `class_hierarchy().member_next_provider(other_q, &SourceMemberKind::Special(kind), other_q, start_from, source)` equals the selected class.
  - This is the provider walk that the native go-to-definition path (`definition::next_dispatch_target` → `class_hierarchy::original_source_member_at`'s native branch) already uses with the same `class.metadata().qualified_name` key.
- Rule check:
  - Every assertion is unchanged, and the patch adds no `#[allow]`.
  - No command names are hard-coded: `next`/`nextto` come from `TCLOO_NEXT_CHAIN` plus the selected `ArgRole::Name`, and the lifecycle kind comes from the registry's `DefinitionSpecialMemberKind`.
  - The patch adds positive spans only, and the coverage stays `ExistingInventory`. Nothing reads absence as completeness.
  - It grants no Native identity or dispatch. The change is readonly geometry on an identity that `original_declaration` already selected.
- Expected results:
  - `via_nextto_explicit_target`: the constructor `nextto Grandparent` in `::Sub` canonicalises to `::Grandparent`, and `constructor_next_provider("::Sub", Some("::Grandparent"))` is `::Grandparent` (body `{ }` is not empty), giving 1 reference. `base_refs` at (4,6) is on the `superclass` line, which this change does not touch, so it stays empty.
  - The braced variant gives the same result, because `literal()` decodes `{Grandparent}`.
  - The plain-`next` constructor/destructor tests and the native constructor/destructor lens counts above should also go green.
- Side effects to review:
  - Native document highlights (`references.rs:4616-4629`) on a lifecycle keyword now also highlight its `next` sites.
  - The server's readonly report path (`readonly_reference_reports.rs:1673`) and `original_call_hierarchy::references` now list them for a lifecycle identity.
  - Rename is not affected: `reference_spans` and `readonly_reference_report` have no edit consumers.
- Not checked statically:
  - The `xtask owner-resolution` / `resolution_drift` gates for `references.rs`. The patch reuses `qualified_name` as a hierarchy key, as `class_hierarchy.rs:200` and `:227` already do.
  - Whether clippy flags `.filter(..).last()`. `declarations()` returns an opaque `impl Iterator`, so `double_ended_iterator_last` should not apply.
- rustfmt (edition 2024, stdin) leaves both patched files unchanged.

---

## 2. Own-object identity: `reference_spans` returns 0 (`original_declaration.rs:2045`)

### What actually happens
- In integration37, the analyser objdefine inline fix (`handlers.rs:6352-6370`) makes the configuration and declaration exist. That is why the test now gets past `:2003` and its first eleven assertions.
- `reference_spans` for `Method` (`original_declaration.rs:886-905`) needs `method_symbol::source_queries` to produce a query for `object p\uD800`. `source_queries` → `command_query` (`method_symbol.rs:428`) → `receiver_identity::method_at_command` (`receiver_identity.rs:239-337`). That function needs a realm own-object receiver entry: one of `receiver_self_method_entry`, `named_object_receiver_method_entry` (`:311`) or `frozen_object_receiver_method_entry`.
- The only producer of own-object entries is `SourceCommandBindings::walk_closed_own_object_method_configuration` (`command_binding/own_object_configuration.rs:48`, called from `command_binding.rs:10597`). It returns `None` unless `native.invocation.arguments().exact_argv_len() == Some(2)` and `facts.arg_roles` contains `(1, Body)` (`own_object_configuration.rs:57-66`).
  - The script form `oo::objdefine object {method …}` passes this check.
  - The inline form `oo::objdefine object method p\uD800 {} {}` has argv length 5, and its layout resolver returns `(4, Body)` (`tcl-registry/src/commands/tcl/oo_objdefine.rs` test `object_target_name_preserves_both_definition_body_shapes`). The producer therefore declines, and the invocation falls back to generic, opaque dispatch handling.
- So `method_at_command` finds no own entry. `class_at_command_head` (`method_symbol.rs:437`) then still finds class `C` for the head `object` and returns `Break(None)`. No query is produced and the count is 0.
- The same cause explains:
  - `original_call_hierarchy::own_object_call_tests::…keep_the_canonical_object_worker` failing at `prepare` (`original_call_hierarchy.rs:926`, Sol's G9 "second cause").
  - Very likely `method_symbol::own_object_tests::original_own_object_generation_and_private_entries_block_class_advice`: its "replacement own worker" comes from an inline `oo::objdefine object method shared …`. That test is not in any capture.

### Why there is no minimal patch
- This is a missing compiler producer form, not a gate in a consumer. A correct fix extends `walk_closed_own_object_method_configuration` to the inline member layout:
  - Accept argv length ≥ 3 when post-head argument 1 is a literal for which `selected.definition_body.is_member(..)` holds. This mirrors the analyser fix's `declared.is_member(keyword)` together with `authored_source_argument_roles().1`. Keep the existing `Configure{layer: Object}` target selection.
  - Build the member "definition" as `ExecutedScriptSource::contiguous(origin, source[argv[2].start .. command end], argv[2].start)`, and only when every member word is a static literal that is byte-identical to its source. A substituted word must decline, as `own_object_method_transfers` already does through `literal_bytes`.
  - Feed that into the existing `own_object_method_transfers`, `install_own_object_method_transfers` and `register_original_method_bodies`.
- What has to be verified at runtime:
  - The analyser's inline `OriginalSourceMethodMetadata` declaration site must equal the realm entry's `declaration()`. `original_own_method_metadata` (`receiver_identity.rs:158-185`) joins on `declaration().site() == entry.declaration()` and on `name_span == entry.name_source().span`. The analyser's inline producer (`original_members.rs` `retain_original_inline_member_metadata`, `producer.member(.., 0)`) appears to anchor at the member keyword, which matches `argv[2].start`, but I could not confirm the offset convention statically.
  - It also needs the correct command end span, given the closing-delimiter convention in AGENTS.md (#527).
  - It changes compiler dispatch proofs. `object_instance.rs:995` and `deferred_method.rs:2578/2637/2846` have inline-objdefine fixtures, and their current expectations look compatible: the inline change still replaces or invalidates the earlier dispatch. They need to be re-run.
- That is a producer extension with a review surface across the compiler, not a minimal consumer fix, so I did not write an unverifiable patch. If you want it, the owner should be whoever owns `own_object_configuration.rs`, and the tests above are the acceptance set.

---

## 3. `project_proc_var_index_for_inputs(&db, project)` is empty (`tcl-lsp-db/src/project_token_inputs/tests.rs:225`)

### What is established
- `project_proc_var_index_for_inputs` (`project_token_inputs.rs:84-102`) collects `checked_file_source_roles_for_config` for every member with `collect::<Option<Vec<_>>>()`. One `None` therefore empties the whole index, which the test intends for the later withdrawal half. `is_empty()` (`semantic_tokens.rs:668-679`) is also true when every provider is present but none `has_roles()`.
- **The empty `other` file is very probably not the cause.** Its `checked_file_token_facts_for_config` passes (test line 213), and that check covers the same `matches_original_source_image` and `resolved_input` conditions (`project_token_inputs.rs:51-56`). Those conditions imply `realm.original_source_image()` is `Some`, because both read `root_origin` (`original_name_value.rs:1661-1671`, `origin_inventory.rs:292`). `command_aliases` is filled only by `interp alias` producers. With no procedures or classes, `capture` (`project_procedure_roles.rs:39-97`) then has no other `?` that can fail.
- **`::ProjectThing create specimen` does not create a `command_aliases` row.** The only producers are `interp alias` (`signature_scan/walker.rs:269`, `signature_scan/handlers.rs:691`, `analyser/handlers.rs:6093`).
- The `proc bind` header works in the passing sibling test with "tcl" structure-only analysis (`proc ::reader`, test 1).
- That leaves the class header in the main file: `analysis.logical_source_class_declaration(record.metadata())?.source_candidate_coordinate()?` (`project_procedure_roles.rs:73-80`), which applies because dialect "tcl" has a logical context.
  - db042 added `retain_logical_class_source_factories` for `structure_only` (`state.rs:2304-2308`, `logical_members.rs:364-403`). Its own control test `structural_source_factory_does_not_require_or_issue_module_body_syntax` is recorded as unexecuted.
  - Candidate failure points, in order of likelihood:
    - `LogicalSourceClassDeclaration::matches` (`logical_members.rs:171-186`) compares a whole `ClassDef` snapshot taken at `retain_original_provider_class` (`oo.rs:703-725`). This is the stale-snapshot hypothesis Sol-2 already raised for Core A.
    - `logical_class_factory` (`logical_members.rs:782-806`) requires `ptr::eq(factory.logical_source_class(self)?, class)`.
    - `logical_source_class` (`source_class.rs:109-…`) has its own source and context checks.
- I could not decide statically between these without running code.

### Deliverable: `probe-3-roles-capture.patch` (diagnostic test, do not commit)
- It adds `sol8_probe_project_roles_capture_steps` to `tcl-lsp-db/src/project_token_inputs/tests.rs`. The test runs the DB's own structure-only analysis on five sources:
  - the empty file
  - the proc alone
  - the class alone
  - the class plus `create`
  - the full test file
- For each one it prints:
  - the logical context
  - the realm image
  - the alias count
  - the registry
  - each class's (logical owner, coordinate)
  - each procedure's logical coordinate
  - whether `capture` succeeded, and `has_roles`
- It fails listing the sources whose capture is None.
- Run it after the capture: `cargo test -p tcl-lsp-db sol8_probe -- --nocapture`. The first `false` names the exact `?`. If the class owner is `false`, look at the snapshot `matches`. If the owner is `true` and the coordinate is `false`, look at `logical_class_factory` and `logical_source_class`.

---

## Files
- `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-8/fix-1-native-lifecycle-next-chain.patch` (a/ and b/ under `a/`, `b/`)
- `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-8/probe-3-roles-capture.patch` (a/ and b/ under `probe/`)

---

## v2 (after Luna-9 HOLD): `fix-1-native-lifecycle-next-chain-v2.patch`

I built this from the same `a/` base, which is still byte-identical to HEAD `bb7277d82` for both files. The trees are under `v2/a` and `v2/b`.

- **B (required), applied.** `original_special_next_chain_spans` returns nothing unless all three of these hold:
  - the selected row is `MemberSide::Instance`;
  - it is the effective (last) row of its kind in its record;
  - its `keyword_word().span()` equals `class_member_def(hierarchy.classes[class_q], Special(kind)).name_span`. This is the live member that native go-to-definition resolves (`definition.rs:1318-1319`).

  This closes S4: destroyed or re-created records share a rendered name, but only the live record's keyword matches. It also closes S5: class-object-side rows are excluded.
- **A (recommended), applied.** For `nextto`, a site is emitted only when:
  - the canonical target equals the selected class;
  - the target sits strictly after the scanning class in `hierarchy.mro_map[other_q]`;
  - the provider walk still returns the selected class.

  The last condition also drops `nextto X` when X's own constructor is empty, because the walk falls through. This closes S1 and S2 for native references. Plain `next` is unchanged.
- **C, left out on purpose.** These need a separate change in `class_hierarchy.rs`, the shared owner:
  - `destructor_next_provider` should treat `destructor {}` as a deletion (S3, confirmed in 8.6.18 and 9.0.4).
  - `constructor_next_provider` and `destructor_next_provider` should resolve `nextto` exactly and check reachability, which would fix the lexical path and native definition too.

  Until then, S3 remains for native references: a subclass `next` can still be attributed to an empty `destructor {}`.
- **Correction (Luna-9 §3).** `original_call_hierarchy::references` is not affected, because its `Special` arm adds nothing at `:818-820`.
- **Residual risk (unverified statically).** The acceptance set needs the ledger keyword span and the `ClassDef.constructors`/`destructor` `name_span` to coincide; both are the keyword word. Luna-9 §4's eight FAIL→PASS tests are that acceptance set.
- **Pre-existing gate.** The `check-name-resolution-proofs` sha256 pins for `references.rs` and `original_declaration.rs` are already stale at HEAD, and they need a refresh before the PR.
- **Checks.**
  - `rustfmt --edition 2024` (stdin) leaves the file unchanged.
  - No `#[allow]` was added.
  - No command names or literals were added; `next`/`nextto` come from the registry (`TCLOO_NEXT_CHAIN` plus `ArgRole::Name`).
  - The patch returns positive spans only.
  - `git -C /home/user/tcl-lsp apply --check` reports OK.
