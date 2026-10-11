# naming.source.retained-positioned-command-realm

Kind: `implementation-contract`

## Problem statement

A Module consumer needs the positioned source interpretation that produced that Module. Rebuilding from printable text, a dialect label or partial entry fields can replace missing original correspondence with a different interpretation. Caller seeds, conditional warnings and in-place rewrites must stop when the retained source owner is unavailable.

## Question

How can every Module consumer reuse the exact positioned source interpretation produced by lowering while preserving source channel, complete grammar, entry, availability generation, namespace and Registry ownership, and how must refusal propagate?

## Conclusion

RetainedSourceModuleBindings keeps one immutable Arc-backed positioned SourceCommandBindings inventory. Its checked projection joins the actual Module source/channel, complete grammar, entry/input, namespace and Registry snapshot to that original producer. document_realm_bindings_for_module shares the derived inventory and complete metadata input without fresh analysis. Refusal makes callers opaque, withdraws interprocedural summaries, withholds conditional warnings and stops rewrites before writes. Actual new cross-file or synthetic source enters its own explicit lowering path. Mutable derived inventory detaches through the central copy-on-write owner.

## Scope

Current Rust source ownership contract and nine genuine direct software test definitions. Their actual module parents, complete bodies and source hashes are inspected; no own executable inventory or assertion outcome is attached. The six-profile software recipe loop is not an external C/Jim comparison. An immutable earlier source owner supplies no correspondence for changed Module bytes, channel, configuration, availability or entry. None from the checked projection is terminal refusal; CommandBindingRealm::none/default and optional helper assistance cannot substitute. Existing Native lookup/evaluation, frame, completion, physical object/store, implementation identity and rewrite guards remain independent. No timing improvement, completed performance comparison, source-to-library equivalence or BIG-IP result is claimed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: No original external-provider process for this Rust source-owner contract. Channel: Current Rust source/API definitions only. Dialect: tcl8.4.

This Rust immutable-source retention and projection contract does not run an external provider.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: No original external-provider process for this Rust source-owner contract. Channel: Current Rust source/API definitions only. Dialect: tcl8.5.

This Rust immutable-source retention and projection contract does not run an external provider.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: No original external-provider process for this Rust source-owner contract. Channel: Current Rust source/API definitions only. Dialect: tcl8.6.

This Rust immutable-source retention and projection contract does not run an external provider.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: No original external-provider process for this Rust source-owner contract. Channel: Current Rust source/API definitions only. Dialect: tcl9.0.

This Rust immutable-source retention and projection contract does not run an external provider.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: No original external-provider process for this Rust source-owner contract. Channel: Current Rust source/API definitions only. Dialect: tcl9.1.

This Rust immutable-source retention and projection contract does not run an external provider.

### jim

Status: `not-tested`. Version: not tested. Build: No original external-provider process for this Rust source-owner contract. Channel: Current Rust source/API definitions only. Dialect: jim.

This Rust immutable-source retention and projection contract does not run an external provider.

### bigip

Status: `not-tested`. Version: not tested. Build: No original external-provider process for this Rust source-owner contract. Channel: Current Rust source/API definitions only. Dialect: bigip.

This Rust immutable-source retention and projection contract does not run an external provider.

## Exact evidence

- `retained-positioned-command-binding` (implementation): [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs). SHA-256 `b457884e90c6a20e065b5e07235eed843683dae103366495d20905721ce31928`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-retained-source` (implementation): [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs). SHA-256 `914ceb69a4e2ab42af4b0ac99ede3a8c3374fc7743090488fe29b7cf0126e181`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-compilation-unit` (implementation): [rust/tcl-compiler/src/compilation_unit.rs](../../../../rust/tcl-compiler/src/compilation_unit.rs). SHA-256 `812947b77edb4b2f1fcd13b6a7b46f4bf737838abedea5c1f4f1ac1c77324cd7`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-inline-uplevel` (implementation): [rust/tcl-compiler/src/inline_uplevel.rs](../../../../rust/tcl-compiler/src/inline_uplevel.rs). SHA-256 `4eafd580f1b015b49f8443a6a1ad2bce15e9ed44747f82aa4c10b93a9dae98e0`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-lib` (implementation): [rust/tcl-compiler/src/lib.rs](../../../../rust/tcl-compiler/src/lib.rs). SHA-256 `00ee19bed242452aadbc7d038e91512a225ce7d5731a14718aa8c6b85dd06759`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-manager` (implementation): [rust/tcl-compiler/src/optimiser/manager.rs](../../../../rust/tcl-compiler/src/optimiser/manager.rs). SHA-256 `3c70e2c9d77675a874a1d87d90cec00e2d7e8ac923b01ab54777ffa08f48f13b`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-mod` (implementation): [rust/tcl-compiler/src/optimiser/mod.rs](../../../../rust/tcl-compiler/src/optimiser/mod.rs). SHA-256 `83faed96f49ebc8e385578f2f27d850d68ab389300fabfebb5aaacd2d77f7932`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-realm` (implementation): [rust/tcl-compiler/src/realm.rs](../../../../rust/tcl-compiler/src/realm.rs). SHA-256 `1ec2575611039c76ce295b3af3440073c74b622baab70413d5b389e25e57d07e`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-taint` (implementation): [rust/tcl-compiler/src/taint.rs](../../../../rust/tcl-compiler/src/taint.rs). SHA-256 `ebfb084d0d2f5387582742f4c4f138852d288a012a544131112e100e847dc576`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.
- `retained-positioned-unit-scope` (implementation): [rust/tcl-compiler/src/unit_scope.rs](../../../../rust/tcl-compiler/src/unit_scope.rs). SHA-256 `fae5c9292d405fee0a52a8d583e50d4fbdbd9581f9d4046fbb7b0614e201f1c2`. Current inspected source owner, consumer or actual parent registration. These bytes bind the software contract only; no producer, listing or passed assertion receipt is attached.

- `own-source347-software-outcomes` (observation): [docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json](../../../../docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json). SHA-256 `903b68fc411ccd67d42d0756077e0fe6df85453fcec7cc3d7619374e33cc9f4c`. JSON pointer `/exact_question_body_outcomes`. Exact original-source347 body/question joins, closed owning producers, strict actual inventories/admissions and per-selector results. Current implementation source and unselected/new controls receive no result from these images.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs), `RetainedSourceModuleBindings::positioned_for_module`: Retain the actual original producer and require complete current Module/source ownership before borrowing.
- [rust/tcl-compiler/src/realm.rs](../../../../rust/tcl-compiler/src/realm.rs), `document_realm_bindings_for_module`: Share positioned source inventory and complete input without fresh interpretation or fallback.
- [rust/tcl-compiler/src/compilation_unit.rs](../../../../rust/tcl-compiler/src/compilation_unit.rs), `resolve_unit_scope`: Use the authentic Module projection; refusal records every reachable local callee as opaque, including future external merges.
- [rust/tcl-compiler/src/compilation_unit.rs](../../../../rust/tcl-compiler/src/compilation_unit.rs), `CompilationUnit::with_interprocedural`: Use authentic original realm, withdraw stale summaries and stop when unavailable.
- [rust/tcl-compiler/src/compilation_unit.rs](../../../../rust/tcl-compiler/src/compilation_unit.rs), `CompilationUnit::with_interprocedural_memoized`: The same Module projection and refusal boundary applies before a taint memo can execute.
- [rust/tcl-compiler/src/optimiser/manager.rs](../../../../rust/tcl-compiler/src/optimiser/manager.rs), `optimise_raw_for_profile`: Build interprocedural analysis from the existing Module original inventory; missing ownership withholds results.
- [rust/tcl-compiler/src/unit_scope.rs](../../../../rust/tcl-compiler/src/unit_scope.rs), `scan_source_call_sites_with_entry`: Use actual freshly lowered Module owner; refusal retracts dispatch-reachable caller seeds.
- [rust/tcl-compiler/src/taint.rs](../../../../rust/tcl-compiler/src/taint.rs), `find_taint_warnings_for_cu_base_with_external_variable_seeds`: Use authentic Module positioned owner before solving and withhold all results on refusal.
- [rust/tcl-compiler/src/taint.rs](../../../../rust/tcl-compiler/src/taint.rs), `find_callback_substitution_warnings`: Lazily retain Module owner or refusal for command-substitution builders; refused candidate never allocates replay names or builds synthetic unit.
- [rust/tcl-compiler/src/inline_uplevel.rs](../../../../rust/tcl-compiler/src/inline_uplevel.rs), `inline_uplevel_passthrough`: Borrow actual positioned producer independently of Module mutations, and terminate before writes on original source-owner refusal.
- [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs), `command_binding::retained_source::tests::positioned_module_projection_shares_the_actual_producer_and_detaches_writes` (linked): The genuine Lowerer-produced positioned inventory and derived copies share the immutable original owner. Copy-on-write mutation detaches the derived state; the retained producer and its subsequent checked projection stay unchanged.
- [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs), `command_binding::retained_source::tests::positioned_module_projection_refuses_each_changed_original_input` (linked): Changes to original source bytes/channel, complete grammar, entry, namespace geometry, compilation scope, loop depth, actual Registry snapshot or foreign Registry withdraw the projection. Matching printable source or a retained earlier entry cannot repair these joins.
- [rust/tcl-compiler/src/realm.rs](../../../../rust/tcl-compiler/src/realm.rs), `realm::tests::original_module_realm_reuses_positioned_inventory_across_native_engine_profiles` (linked): Six selected C/Jim Rust engine-profile recipes reuse their actual Lowerer inventory, complete source origin and derived positioned data; an explicit fresh source ingress supplies its own equal model. This loop is software source inspection, not six external-provider observations.
- [rust/tcl-compiler/src/realm.rs](../../../../rust/tcl-compiler/src/realm.rs), `realm::tests::original_module_realm_refuses_missing_and_same_store_foreign_availability` (linked): Missing retained owner/input and older availability from the same command-store allocation refuse the projection. Updating entry metadata to that older availability does not revive the original owner; the current original input remains unchanged.
- [rust/tcl-compiler/src/compilation_unit.rs](../../../../rust/tcl-compiler/src/compilation_unit.rs), `compilation_unit::tests::module_realm_refusal_withholds_caller_seeds_and_retires_interprocedural_results` (linked): Missing or changed Module source ownership makes reachable callers opaque and withholds uniform caller seeds. Ordinary and memoized interprocedural consumers withdraw summaries; the refused memoized route invokes no taint memo.
- [rust/tcl-compiler/src/unit_scope.rs](../../../../rust/tcl-compiler/src/unit_scope.rs), `unit_scope::tests::independent_caller_ingresses_keep_their_actual_sources_and_loader_identity` (linked): Two genuine independent caller ingresses retain their own new source and literal caller evidence while preserving the actual loader implementation identity. Their fresh lowering is independent of a refused existing Module query.
- [rust/tcl-compiler/src/taint.rs](../../../../rust/tcl-compiler/src/taint.rs), `taint::tests::whole_unit_warning_query_requires_its_retained_positioned_source_owner` (linked): A genuine whole-unit warning query returns its conditional warning under the retained source owner and returns no warnings for missing owner, changed original source or missing Registry snapshot. The original positive remains unchanged.
- [rust/tcl-compiler/src/taint.rs](../../../../rust/tcl-compiler/src/taint.rs), `taint::tests::command_substitution_callbacks_stop_before_replay_when_original_owner_is_refused` (linked): A genuine selected command-substitution callback creates its conditional warning through one replay. Missing or changed original Module ownership stops before replay-name search or synthetic compilation. Authored Tk source provenance supplies no external callback execution.
- [rust/tcl-compiler/src/inline_uplevel.rs](../../../../rust/tcl-compiler/src/inline_uplevel.rs), `inline_uplevel::tests::rewriter_uses_the_original_positioned_owner_and_stops_before_writes_on_refusal` (linked): A genuine model uplevel candidate can rewrite through its retained original producer. Missing owner, changed source/configuration or missing snapshot refuses before statements move and preserves the complete Module. Independent callsite, implementation, frame and completion guards remain required.
- [rust/tcl-compiler/src/realm.rs](../../../../rust/tcl-compiler/src/realm.rs), `realm::original_resolved_realm_input_tests::original_resolved_realm_producers_keep_exact_input_and_independent_source_currency` (linked): Finite source-defined control: original resolved realm producers keep exact input and independent source currency. Retaining editing input supplies no Native admission or frame entry. Same exact input is independent of source-byte/channel currency. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-compiler/src/realm.rs](../../../../rust/tcl-compiler/src/realm.rs), `realm::original_resolved_realm_input_tests::original_resolved_realm_producers_refuse_missing_foreign_and_changed_inputs` (linked): Finite source-defined control: original resolved realm producers refuse missing foreign and changed inputs. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-compiler/src/taint.rs](../../../../rust/tcl-compiler/src/taint.rs), `taint::original_taint_local_parser_purpose_tests::original_taint_callback_and_bracket_shapes_keep_actual_full_config` (linked): Finite source-defined control: original taint callback and bracket shapes keep actual full config. Genuine full Module/FU input passes full selected lexerconfig into local callback/bracket shape projections; nested BOM, FirstClose/Nesting, stale and missing actual source metadata controls. Synthetic source cannot create execution receipts. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

A named test is a coverage binding, not a claim that it executed.

## Replay

Source definitions only. A matching independently successful own producer, exact executable inventory and complete assertion receipt are required for executed software outcomes. Actual independent source ingress and every Native/provider/frame/completion/edit receipt remain separate. Complete original/current measurement receipts are required for performance claims.


## Exact original-source Compiler software outcomes

The table reports only admitted selectors attached to this question by exact comments in the retained original test bodies. Each result is limited to its named source snapshot, owned Compiler image and selector invocation. No other source/image, unselected assertion, external-provider observation or performance conclusion follows.

| Original operation | Passed | Failed | Incomplete |
| --- | ---: | ---: | ---: |
| `integration-compiler-focused-consumer-tests344` | 4 | 0 | 0 |

[Exact original-source body hashes, receipts, streams, inventories and owned images](rust-validation/compiler-owned341345-experiments.json) preserve the per-selector events and independent producer/admission limits. The existing provider answers retain their own experimental scope.


## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-compiler-focused-consumer-tests347` | 4 | 0 |

[Exact original body hashes, producer/list/admission receipts, raw logs and owned images](rust-validation/compiler-core-runtime-owned347-experiments.json) retain the finite per-selector results and independent failed Runtime producer.
