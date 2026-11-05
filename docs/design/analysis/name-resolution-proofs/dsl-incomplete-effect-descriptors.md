# naming.dsl.incomplete-effect-descriptors

Kind: `implementation-contract`

## Problem statement

A SpecTcl block may contain a recognized outer semantic field with inner rows the loader cannot represent. Dropping these rows and returning an empty descriptor tells analysis that no effect occurs, even though the declared effect was never loaded. The loader and Studio renderer need to expose this parity gap truthfully.

## Question

Do unsupported semantic rows and invalid composition values withdraw strong command admission instead of becoming an empty effect or transition descriptor?

## Conclusion

Unsupported world_effects/state_transitions rows and invalid composition values mark the descriptor incomplete and exclude the command from strong analysis. Supported composition-only blocks and explicit world_effects none remain valid. Studio records these inner typed rows as a representational gap; unknown data is not silently promoted to EMPTY.

## Scope

Rust implementation contract under the explicit contexts used by the linked tests. This record makes no C Tcl, Jim or BIG-IP observation claim, no Native entry claim and no successful evaluation claim.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

## Consumer bindings

- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `state_transitions_value`: Own semantic completeness before admission.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `world_effects_value`: Keep incomplete data separate from explicit empty semantics.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `loader::tests::incomplete_effect_and_transition_blocks_cannot_claim_empty_semantics` (linked): Reject unsupported transition/world rows and invalid composition, while retaining valid composition and explicit empty world effects.

- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `loader::tests::state_transition_blocks_read_every_row_world_effects_only_composition_issue_2140` (linked): The composition-only control preserves Replace and resolver none. Every row in the fixed malformed-transition roster is read and suppresses command publication instead of exposing a partial descriptor; the unsupported-world-effect roster likewise suppresses command publication.
