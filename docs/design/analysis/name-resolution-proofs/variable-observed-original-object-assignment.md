# naming.variable.observed-original-object-assignment

Kind: `implementation-contract`

## Problem statement

An original name-object assignment can bypass a positively selected observed naming purpose and use a default receiver path, losing the complete counted name or its actual entered event frame.

## Question

How do original name-object reads and assignments retain the measured naming purpose, complete original bytes and current authentic Rust event-frame receiver?

## Conclusion

For the positively selected observed purpose, original reads and assignments retain complete original bytes and delegate the existing entered-frame name receiver instead of the Jim cache route. Assigning without result publication and storing with result publication keep their separate APIs. Scalar and element spellings remain distinct from their prefix values; a retired event frame cannot supply a receiver. Native C/Jim policy and measured-purpose selection guards remain independent.

## Scope

One authored Rust allocation/receiver selector with two counted scalar/element forms, separate prefix values, assignment/result/read checks and retired-frame refusal. Actual event-frame ownership is an internal Rust premise, not an appliance frame measurement. No native provider execution, arbitrary observed name, Native cache/header/pointer/layout, new naming policy, compiler admission or source-model Normal grant is supplied. Existing BIG-IP grammar observations retain their separate exact input purposes.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No observation for this implementation question.

## Consumer bindings

- [runtime/rust/src/interp/native_jim_lookup.rs](../../../../runtime/rust/src/interp/native_jim_lookup.rs), `Interp::read_original_named_variable`: Retain complete original bytes and the independently selected observed entered-frame receiver before Jim cache fallback; preserve result-publication distinction.
- [runtime/rust/src/interp/native_jim_lookup.rs](../../../../runtime/rust/src/interp/native_jim_lookup.rs), `Interp::assign_original_named_variable`: Retain complete original bytes and the independently selected observed entered-frame receiver before Jim cache fallback; preserve result-publication distinction.
- [runtime/rust/src/interp/native_jim_lookup.rs](../../../../runtime/rust/src/interp/native_jim_lookup.rs), `Interp::store_original_named_variable`: Retain complete original bytes and the independently selected observed entered-frame receiver before Jim cache fallback; preserve result-publication distinction.
- [runtime/rust/src/interp/execution_name_policy.rs](../../../../runtime/rust/src/interp/execution_name_policy.rs), `interp::execution_name_policy::tests::original_object_assignments_use_the_current_observed_receiver` (linked): A genuine authored Rust event frame keeps scalar/element counted names separate from prefix values. Assignment leaves the current result unchanged, store publishes the supplied value, reads retain actual Rust object identity, and the retired frame refuses. No native private layout is measured.
