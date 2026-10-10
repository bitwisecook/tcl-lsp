# naming.tcloo.original-relation-provider-fold-contract

Kind: `implementation-contract`

## Problem statement

Superclass and mixin operands can contain opaque counted units, and different operand spellings can select the same class provider. Folding names as Strings before provider lookup can collapse distinct classes or fail removal through another spelling. Class-object mixins must also remain independent of instance inheritance.

## Question

Does the original class relation ledger retain counted operands and caller lookup scope, then fold joined provider identities through one Registry slot owner on the selected receiver axis?

## Conclusion

OriginalSourceClassRelationLedger retains the original operand input, lookup geometry, full operation site, selected receiver side and ordered effect. Its resolve callback joins every operand before SlotSpec applies the operation to provider identities. Unknown operands or flags withdraw the affected list until an exact replacement or clear renews it. Instance superclass/mixins and class-object mixins remain separate. The ledger supplies source metadata and cannot prove provider existence, successful TclOO execution or native MRO.

## Scope

Current Compiler/Registry implementation contract for original TclOO source relation operands. The regressions distinguish two opaque surrogate-unit names in a namespace, instance versus class-object mixins, different spellings joined to one provider, and replacement after an unresolved operand. Rust execution status is unverified here; no C Tcl, Jim or BIG-IP execution is claimed by this record.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source-metadata/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source-metadata/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source-metadata/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source-metadata/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust source-metadata/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This is a Rust source-metadata/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This is a Rust source-metadata/provenance contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Exact evidence

- `owner-0` (implementation): [rust/tcl-compiler/src/analyser/types/original_relations.rs](../../../../rust/tcl-compiler/src/analyser/types/original_relations.rs). SHA-256 `2af4ce37663a459061e11126010e05d614c3dde386474da379e7c218714646ee`. Inspected current shared producer, readonly metadata and regression assertion source; this evidence supplies no native execution outcome.

- `owner-1` (implementation): [rust/tcl-compiler/src/analyser/original_members.rs](../../../../rust/tcl-compiler/src/analyser/original_members.rs). SHA-256 `6c5be879ccd0cb82b1b19c440886c1e097199b6a80ffa73e2078c888c92e06f4`. Inspected current shared producer, readonly metadata and regression assertion source; this evidence supplies no native execution outcome.

- `owner-2` (implementation): [rust/tcl-registry/src/definer.rs](../../../../rust/tcl-registry/src/definer.rs). SHA-256 `3b92e404d765529848fb0eeec9a58d12173fa5314aab274cd9e20961405e2ac9`. Inspected current shared producer, readonly metadata and regression assertion source; this evidence supplies no native execution outcome.

## Source inspection

No native implementation source excerpt is attached. Native outputs cannot establish this Rust provenance invariant.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/types/original_relations.rs](../../../../rust/tcl-compiler/src/analyser/types/original_relations.rs), `OriginalSourceClassRelationLedger::resolve`: Joins independently selected providers before the shared Registry slot fold; it supplies no provider or execution grant.

- [rust/tcl-compiler/src/analyser/original_members.rs](../../../../rust/tcl-compiler/src/analyser/original_members.rs), `analyser::original_members::tests::original_relations_preserve_opaque_operands_caller_scope_and_receiver_side` (linked): Checks the independently retained original inputs and allocation/provider/receiver-axis boundary stated in this contract.

- [rust/tcl-compiler/src/analyser/original_members.rs](../../../../rust/tcl-compiler/src/analyser/original_members.rs), `analyser::original_members::tests::original_relation_removal_compares_joined_providers_and_renews_after_unknown` (linked): Checks the independently retained original inputs and allocation/provider/receiver-axis boundary stated in this contract.

A named test is a coverage binding, not a claim that it executed.

## Replay

Execute the named selectors under the coherent Rust workspace before claiming a passing result. No native replay is attached to this implementation contract; source changes require evidence reissue.
