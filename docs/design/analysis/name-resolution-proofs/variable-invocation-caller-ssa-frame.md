# naming.variable.invocation-caller-ssa-frame

Kind: `implementation-contract`

## Problem statement

An invocation can retain physical reads from a callee activation as well as caller and shared cells. Treating every subtree read as a caller-local SSA use creates an unrelated caller dependency; a display label cannot identify the activation that owns a retained cell.

## Question

How does caller SSA retain its actual activation reads and shared cells while preserving the complete physical invocation inventory and excluding foreign callee-local dependencies?

## Conclusion

VariableCellKey::activation_identity queries the typed original root, including lifetime/element wrappers and retained variable slots. Authored presentation names do not acquire an activation by parsing their labels. invocation_read_bindings retains actual original invocation execution/argument places, projects canonical value-cell keys and compares any retained activation identity with the caller ResolveContext. Foreign callee activation cells do not enter caller SSA; caller activation, shared namespace and genuine root-alias cells remain. The original physical subtree inventories remain available independently. This dependency projection neither suppresses diagnostics directly nor changes runtime reads or cells. Phi and nonlocal invocation read projections supply the authentic before-statement or before-terminator caller ResolveContext to this same typed activation query.

## Scope

Three linked authored Rust controls: retained-slot lifetime/element activation identity versus equal authored labels and namespace cells; actual caller versus callee/global context keys; two fixed source invocations retaining a callee physical read separately from caller SSA and preserving the root alias read. Exact retained types and caller ResolveContext own the projection. Source/display strings grant no physical allocator fact, Native name/entry, actual caller-frame entry, current value, Normal completion or edit permission. Native provider answers and separately retained lifecycle/regression executions remain independent.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This typed caller-SSA projection question has no native provider execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This typed caller-SSA projection question has no native provider execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This typed caller-SSA projection question has no native provider execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This typed caller-SSA projection question has no native provider execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This typed caller-SSA projection question has no native provider execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This typed caller-SSA projection question has no native provider execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This typed caller-SSA projection question has no native provider execution receipt.

## Exact evidence

No evidence artifact is attached.

- `own-source347-software-outcomes` (observation): [docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json](../../../../docs/design/analysis/name-resolution-proofs/rust-validation/compiler-core-runtime-owned347-experiments.json). SHA-256 `903b68fc411ccd67d42d0756077e0fe6df85453fcec7cc3d7619374e33cc9f4c`. JSON pointer `/exact_question_body_outcomes`. Exact original-source347 body/question joins, closed owning producers, strict actual inventories/admissions and per-selector results. Current implementation source and unselected/new controls receive no result from these images.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/var_resolve/cell_key.rs](../../../../rust/tcl-compiler/src/var_resolve/cell_key.rs), `VariableCellKey::activation_identity`: Borrow an activation only from typed actual cell-root or retained variable-slot ownership; label rendering remains separate.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `invocation_read_bindings`: Project original invocation subtree read keys into the actual caller SSA context without mutating the physical inventory.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `invocation_read_key_in_context`: Compare retained activation identity to caller ResolveContext; shared namespace and non-activation keys remain independently valid.
- [rust/tcl-compiler/src/var_resolve/cell_key.rs](../../../../rust/tcl-compiler/src/var_resolve/cell_key.rs), `var_resolve::cell_key::tests::activation_identity_uses_retained_structures_instead_of_display_labels` (linked): Lifetime/element wrapping preserves an actual retained variable-slot activation; an equal authored compatibility label and namespace key acquire no activation identity.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `ssa::cell_resolution_tests::caller_ssa_read_key_uses_typed_activation_identity` (linked): An actual caller activation retains its wrapped key; foreign callee keys decline in caller/global contexts, authored labels remain compatibility keys and shared namespace cells survive.
- [rust/tcl-compiler/src/ssa.rs](../../../../rust/tcl-compiler/src/ssa.rs), `ssa::cell_resolution_tests::invocation_read_projection_keeps_physical_inventory_and_root_alias_reads` (linked): Two fixed original source programs retain physical callee subtree reads independently while caller SSA excludes foreign activation locals and preserves a genuine #0 root-alias read.
- [rust/tcl-compiler/src/place_bridge/original_read_source.rs](../../../../rust/tcl-compiler/src/place_bridge/original_read_source.rs), `place_bridge::original_read_source::tests::original_cfg_argument_phase_retains_owner_and_missing_input_clobbers_all_consumers` (linked): Finite source-defined control: original cfg argument phase retains owner and missing input clobbers all consumers. Genuine foreach CFG EvaluatedArguments producer retains actual input once; withdrawing input preserves unknown Place and Quoted SSA liveness, memory wildcard/clobber, and cell-state unknown clobber. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.

A named test is a coverage binding, not a claim that it executed.

## Replay

Three body-marked Rust selectors are authored coverage bindings. Their executed results require independently retained exact-source receipts; no native observation or successful Rust result is attached to this caller-SSA contract.


## Exact original-source Compiler software outcomes

The table reports only admitted selectors attached to this question by exact comments in the retained original test bodies. Each result is limited to its named source snapshot, owned Compiler image and selector invocation. No other source/image, unselected assertion, external-provider observation or performance conclusion follows.

| Original operation | Passed | Failed | Incomplete |
| --- | ---: | ---: | ---: |
| `integration-compiler-focused-consumer-tests344` | 1 | 0 | 0 |

[Exact original-source body hashes, receipts, streams, inventories and owned images](rust-validation/compiler-owned341345-experiments.json) preserve the per-selector events and independent producer/admission limits. The existing provider answers retain their own experimental scope.


## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-compiler-focused-consumer-tests347` | 0 | 1 |

[Exact original body hashes, producer/list/admission receipts, raw logs and owned images](rust-validation/compiler-core-runtime-owned347-experiments.json) retain the finite per-selector results and independent failed Runtime producer.
