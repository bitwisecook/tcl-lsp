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

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `state_transitions_value`: Own semantic completeness before admission.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `world_effects_value`: Keep incomplete data separate from explicit empty semantics.
- [rust/tcl-spectcl/src/loader.rs](../../../../rust/tcl-spectcl/src/loader.rs), `loader::tests::incomplete_effect_and_transition_blocks_cannot_claim_empty_semantics` (linked): Reject unsupported transition/world rows and invalid composition, while retaining valid composition and explicit empty world effects.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "--manifest-path",
  "rust/Cargo.toml",
  "loader::tests::incomplete_effect_and_transition_blocks_cannot_claim_empty_semantics"
]
```

Run each listed selector in its owning crate; the argument vector shows the first selector. A coverage binding is not a recorded test execution. Native interpreter measurements are separate records.
