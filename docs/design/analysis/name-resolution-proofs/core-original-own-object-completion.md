# naming.core.original-own-object-completion

Kind: `implementation-contract`

## Problem statement

One allocated instance can override a class method while a sibling still uses the class entry. The class-object table, reporting object label and final configuration walk cannot identify the current own method. Applicability needs the actual post-argument receiver/own-table generation and a separate canonical declaration join.

## Question

Do own-object completions use the exact current own table and canonical original configuration metadata?

## Conclusion

Own completion consumes the retained actual object allocation and current own-table receipt, joins each entry by allocation plus original declaration site/name span, and uses current entry name bytes and visibility independently of canonical body metadata. Private own identities block identical class candidates. Missing canonical metadata or unknown generations withdraws the combined candidate view; class declaration advice retains its separate actual receiver/class prerequisite. No absence, winning MRO, body execution or per-object variable inventory is inferred.

## Scope

Core method completion for independently retained bounded source-created stock objects and closed ordinary own setters. Siblings, opaque names, unknown setters and stale complete source remain distinct. Missing own receipt is not an own-table absence proof.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This Rust consumer contract has no native execution receipt.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This Rust consumer contract has no native execution receipt.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This Rust consumer contract has no native execution receipt.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-lsp-core/src/receiver_identity.rs](../../../../rust/tcl-lsp-core/src/receiver_identity.rs). SHA-256 `7ee73ce5c7a92ac9ad58001b24568944442147569fe8ade37c2a346b1cbbbdc7`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.
- `implementation-1` (implementation): [rust/tcl-lsp-core/src/original_oo.rs](../../../../rust/tcl-lsp-core/src/original_oo.rs). SHA-256 `bad9d447db474c98dae922606d0bd8a0638d9d677b6f41832f30187f339b873c`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.
- `implementation-2` (implementation): [rust/tcl-lsp-core/src/completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs). SHA-256 `f6a7d8703f8a126955801b30385478f95473e3ec23fcccfe58fd738ed3b00dca`. Current shared owner, consumer and fixed validation assertions. No executed Rust or native result is implied.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/receiver_identity.rs](../../../../rust/tcl-lsp-core/src/receiver_identity.rs), `original_own_method_metadata`: Reborrow one exact own configuration declaration without current dispatch or visibility donation.
- [rust/tcl-lsp-core/src/original_oo.rs](../../../../rust/tcl-lsp-core/src/original_oo.rs), `own_object_completion_methods`: Consume authentic positioned head and current own-table allocation/generation.
- [rust/tcl-lsp-core/src/completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `original_instance_method_candidate_items_at`: Layer exact own identities over separate class declaration candidates.
- [rust/tcl-lsp-core/src/completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `completion::original_own_object_completion_tests::original_own_object_completion_keeps_allocations_and_opaque_routes_after_reporting_clear` (linked): Own and sibling allocations stay separate and two opaque insertions survive reporting collapse.
- [rust/tcl-lsp-core/src/completion.rs](../../../../rust/tcl-lsp-core/src/completion.rs), `completion::original_own_object_completion_tests::original_own_object_completion_requires_the_actual_generation_and_canonical_declaration` (linked): Unknown setters and absent canonical configuration metadata cannot borrow class/UI members.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named fixed Rust selectors bind validation obligations. No Rust execution receipt is attached to this implementation contract. No native interpreter experiment or overall passing suite is claimed. Missing authority and unsupported source purposes remain explicit in the scope and conclusion.
