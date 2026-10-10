# naming.registry.return-identity-effect-coverage

Kind: `implementation-contract`

## Problem statement

A return that only publishes result/completion metadata should not lose current identity coverage through an absent transition descriptor. Error-option publication and unknown argument/dialect envelopes can invoke observers, so those invocations cannot share a closed empty identity-effect summary.

## Question

How does the selected original return state envelope retain independent identity-effect coverage without treating unknown observer or error-option effects as absent?

## Conclusion

The existing native_return_state_effect owner independently selects ResultAndCompletion or MayMaterialiseError from the actual original argument envelope and dialect. The Registry return identity descriptor is closed empty only for ResultAndCompletion. MayMaterialiseError retains a whole-invocation widening across every tracked identity domain with possible commit before abrupt completion. Its wildcard subject is unlocated metadata: it has no original operand ordinal, name input, affected physical cell or source edit authority. Identity coverage remains separate from result bytes, callback footprint, completion code and handler execution.

## Scope

Selected Registry return identity-effect metadata and fixed argument/dialect envelope controls. Dynamic result-only words retain independent value uncertainty; error/options/unknown operands or missing dialect retain observer and whole-invocation identity widening. No Rust execution receipt, native observation, arbitrary callback inventory, handler Normal or writable source subject is issued.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/registry.rs](../../../../rust/tcl-registry/src/registry.rs), `native_return_state_effect`: Select the original argument/dialect result-only versus error-publication envelope independently of value and completion certainty.
- [rust/tcl-registry/src/commands/tcl/return_.rs](../../../../rust/tcl-registry/src/commands/tcl/return_.rs), `return_state_transitions`: Issue closed empty identity transitions only for the selected result/completion envelope and preserve unknown error-option observer effects.
- [rust/tcl-registry/src/state_transition.rs](../../../../rust/tcl-registry/src/state_transition.rs), `StateTransitions::unknown_invocation`: Retain unlocated whole-invocation widening across every identity domain without inventing an original name or operand coordinate.
- [rust/tcl-registry/src/completion_route.rs](../../../../rust/tcl-registry/src/completion_route.rs), `completion_route::tests::return_state_envelope_preserves_error_observers_and_dynamic_results` (linked): All five selected C dialects keep result/completion-only identity facts closed empty independently of dynamic result values; error/options and missing dialect widen every domain with possible pre-abrupt commit while observer effects remain independent.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact linked Rust selector exercises the authored envelope and identity-effect premises. No Rust execution receipt or native provider observation is attached to this implementation contract.
