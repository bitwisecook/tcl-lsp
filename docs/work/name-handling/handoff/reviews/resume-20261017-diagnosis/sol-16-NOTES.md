# Sol-16: `lsp-semantic-logical-receiver-selector` (worklist #9)

Static review only. Nothing in `/home/user/tcl-lsp` was changed, and no cargo build or test was run.
Base: `HEAD` = `a146f2608`. Its relevant source matches capture41's `d0cd9d73b`: no file under `rust/tcl-lsp-core/src/semantic_tokens*`, `rust/tcl-compiler/src/registry_invocation*` or `rust/tcl-compiler/src/object_types*` changed in between.

## Deliverables

| File | Purpose |
|---|---|
| `sol-16-receiver-collection.patch` | Proposed patch: `diff -ruN a b`, 2 files. `git apply --check` against HEAD passes. |
| `sol-16-probe-NOT-FOR-ADMISSION.patch` | Optional `#[cfg(test)]` trace, applied on top of the patch above and gated by `TCL_LSP_PROBE_RECEIVER_SOURCE`. Use it only to find the gate that refuses in the next execution window. Never admit it. |
| `a/`, `b/`, `c/` | Trees from `git show HEAD:<path>`: `b` is HEAD plus the patch, `c` is `b` plus the probe. |

## Catalogue row (verbatim, `repository-consumer-audit-final.json` `units[581]`)

- id `lsp-semantic-logical-receiver-selector`. Site: `rust/tcl-lsp-core/src/semantic_tokens/receiver_source.rs:38 OriginalLogicalReceiverClass::insert_method`.
- purpose: "Method/accessor presentation after genuine conditional class-source receipt."
- api: "Actual held class metadata/grammar and hierarchy metadata match; command.texts.get(1) representative String selects method/property/builtin role."
- required_change: "Method selector lacks original wholeword/static/effective-argv role proof; report String/hierarchy map can classify a compound/dynamic first fragment. Needs shared source member/receiver method operand owner. Object collections lookup similarly uses scalar source name text→ObjectClassMap without typed frame/source occurrence key; actual map producer scope pending main semantic audit. No Native identity grant claimed."
- implementation.status: `partially_implemented_pending_validation`. scope: "Actual full recorded command/static selector reborrows shared original receiver/member owner before highlighting. Legacy collection map occurrence/frame identity remains open."

## The six failures (capture41, `original-tcl_lsp_core`)

Every failing assertion is a positive check that a `Method` token is present. None of them reaches the collection lookup.

| log | selector | failing line | receiver form | path taken |
|---|---|---|---|---|
| test-473 | `original_receiver_heads_keep_canonical_class_and_actual_availability` | tests.rs:63 (first case `[::C new] ping`, tcl8.6) | direct constructor | `candidates()` → `LogicalClass` → `insert_method` |
| test-474 | `original_registry_and_collection_receivers_keep_selected_child_operands` | tests.rs:87 `[listbox .l] curselection` | Registry widget factory | `candidates()` → `ClassNames` → `registry.instance_method_at` |
| test-475 | `original_receiver_class_receipt_cannot_be_replaced_by_foreign_hierarchy_metadata` | tests.rs:139 `[C new] ping` | direct constructor | LogicalClass |
| test-476 | `original_receiver_heads_keep_actual_leading_bom_policy` | tests.rs:192, `LeadingBom::Skip` case (left false, right true) | direct constructor | LogicalClass |
| test-477 | `original_receiver_public_tokens_refuse_populated_unavailable_and_stale_source` | tests.rs:202 `[C new] ping` | direct constructor | LogicalClass |
| test-478 | `logical_receiver_method_requires_original_static_whole_selector` | tests.rs:254 (first case `ping`) | direct constructor | LogicalClass |

The collection assertions in test-474 (tests.rs:122–130, `dict set items key [C new]; [dict get $items key] ping`) come after line 87 and never run. **The scalar-text collection lookup does not cause any of the six recorded failures.** Replacing it cannot by itself flip any of these selectors.

History: all six fail in every capture from capture21 through 36 and 41. The last recorded pass is frozen320 (head `e5bcb153c`). That commit is not in this shallow history, and 1,115 Rust files differ between its source manifest and the root commit `4c717119b`, so no static bisect is possible. `logical_receiver_method_requires_original_static_whole_selector` has never passed; commit `3d9261462` introduced it together with the `insert_method` reborrow.

### Gates the failing paths cross (static trace)

These are cleared by reading the code:

- `ctx.lexical` is true for these inputs. `plain_tcl` has no `core_point` or runtime version, so `has_logical_source_name_context` holds, and there is no `SourceAnalysisEntry`.
- `original_roles.has_head([offset])` is false for a `[...]` head: a Logical head key requires a static word, `original_name.rs:166`. So `classes` reaches `insert_method`.
- `[C new]`: the `C` cell is `AdviceCell::SourceClass`, which sets a Registry barrier (`source_transition_advice.rs:1765`). `source_registry_words_at` therefore returns `None`, and `candidates()` falls through to the constructor-call path.
- `hierarchy.classes[::C]` and `logical_source_class` are both clones of `all_classes`, so they compare equal.

These cannot be decided statically. They are where the probe patch writes its trace:

1. `candidates()` (`receiver_source.rs:117–163`): `original_logical_source_words_at`; the head-word `NativeWord` equality `words.head_source().word() == child_head`, which is listbox only; `call.original_words() == child.command().words`; `logical_constructor_shape`; `logical_source_class`.
2. The `insert_method` reborrow (`project_class_cards.rs:410`): `binding.original_recorded_command()` at the outer `[` offset with strict segment equality; `constructor_receiver_class`, which requires `OriginalProjectClassCards::capture`, `capture_class` (→ `OriginalInstanceClassMetadata::capture` → `logical_source_class_declaration` with `ptr::eq` and full `ClassDef` equality), `ptr::eq` on the factory, and `finish`.
3. Listbox: `instance_method_at("listbox", "curselection", floor, authoring_query)` under the tests' tcl8.6 context.

A weak related signal: `type_hierarchy::known_relation_selection_receipt_tests::ordinary_relation_keeps_original_operand_and_rejects_changed_receiver` (test-434) fails at `analysis.logical_source_class_at(...).is_some()`. The Logical method-implementation tests (test-444, 445, 450) also fail. That points to a shared upstream refusal in the Logical class-receipt chain listed under gate 2. It is a hypothesis, not proved.

## What the patch does

Recorded open obligation: "Legacy collection map occurrence/frame identity remains open."

1. `receiver_source.rs`
   - Removes `collection_classes` and `scalar_reference_name`. Together they read the variable text of `$items` and looked it up in the document-wide `ObjectClassMap`, which has no frame, occurrence or shadow identity.
   - The routing check `element_extraction` stays. It is the same Registry data as before (`ReturnElements::ElementOf` plus the exact-argv check for inner selection), but it now only routes, through the new `ReceiverSourceCandidates::CollectionElement`.
   - New `insert_collection_method` reborrows `original_project_receiver_class_with_sources(…, &[], ctx.original_collections)`. The collection owner keys element classes by `SignatureSourceVariableOccurrence` and by its place, a variable symbol or a declared field. The method is marked only when `collection_source()` is present, every represented element agrees on one factory and metadata, and `provides_method()` holds on the original selector word.
2. `semantic_tokens.rs`
   - `OriginalObjectCollectionSources` is now captured whenever a CU exists and the walk is Logical (`lexical`) or project mode. Before, it was captured in project mode only, which is why the non-project Logical path had only the scalar map.
   - Drops the now-unread `ScriptCtx::object_collections` field. The local map still feeds `augment_loop_var_handles`.
   - Adds the match arm for the new variant.

Invariants:

- Registry data only, with no command names.
- No `#[allow]`.
- No assertion, selector, fixture or time budget changed.
- A missing CU or an unrepresented or ambiguous association gives no hint, and the owner keeps `has_unknown_residual()`. Absence is never read as completeness.
- Source-only owner with `providers = &[]`: no Native identity grant.
- No Source SSA identity is touched.

Behaviour change: in Logical non-project mode, a collection element that the typed owner cannot represent now abstains where the name map used to guess. Examples are Registry-factory elements, `dict append`, and associations the CU capture cannot place. This fails closed by design. The two existing collection fixtures, `semantic_tokens::tests::tcloo_dispatch_pattern_fixture` (case `dict_collection`) and `collection_dispatch_resolves_user_configurable_method`, run under tcl9.0. tcl9.0 uses Native names, so `lexical` is false, and those fixtures never enter this path. Cost: one extra `OriginalObjectCollectionSources::capture` per token request in Logical documents that supply a CU.

## Expected flips

- **None of the six can be claimed.** Their failing lines come before, or are independent of, the collection lookup.
- test-474's collection half (lines 122–130): if line 87 is fixed, the positive at line 123 should pass through the typed owner. This is the same path the compiler fixture `project_class_collection_keeps_original_insertion_extraction_and_unknown_contents` asserts. The negative at 125–130 stays refused: the test passes no CU, so there is no source, and the inner index fails the exact-argv check. These parts are not executed.
- The patch must still go through `make rust-check` (fmt, clippy pedantic) and the six selectors plus the semantic-token suites, under the unchanged 180 s and 30 s budgets.

## Closure claim

**The patch does not fully close the row.**

- It closes, statically and pending execution, the recorded open obligation "Legacy collection map occurrence/frame identity" for this consumer site: the collection receiver now goes through the typed frame/source occurrence owner, and no scalar text is involved.
- These still keep the row PARTIAL:
  - The six selectors still fail for an upstream cause the static trace could not isolate.
  - `insert_method` still uses the `command.texts.get(1)` String for `property_accessor_methods`, `method_target` and the configure-option presentation. It is now behind the original-word reborrow, but the API in the catalogue's `api` field remains.
  - The `ObjectClassMap` producer still feeds the `$var` and loop-variable handle maps. The catalogue explicitly leaves that "actual map producer scope pending main semantic audit".
  - The audit row needs a fresh executed validation.

## Next actions

1. In the next guarded build, apply the patch and then the probe. Run the six selectors with `TCL_LSP_PROBE_RECEIVER_SOURCE=1 --nocapture`. The `RECEIVER_SOURCE_PROBE` lines name the refusing gate: `collect_script.lexical`, `lexical_overrides.has_head`, `candidates=Err(…)` or `Ok(…)`, `insert_method.hierarchy` / `hierarchy_metadata` / `reborrow` / `receiver`, and `collection.receiver`.
2. Repair the gate it names in its owner, then drop the probe.
3. Also run test-434, 444, 445 and 450 to confirm or refute the shared Logical class-receipt hypothesis.
