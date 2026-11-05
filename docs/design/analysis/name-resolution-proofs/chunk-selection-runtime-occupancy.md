# naming.compiler.chunk-selection-runtime-occupancy

Kind: `implementation-contract`

## Problem statement

An earlier source command can make later runtime command occupancy unavailable even though the immutable chunk compiler snapshot already selected the original constant head. Requiring a proved runtime handler to retain that compiler selection erases independent compile-time metadata. Conversely, retaining compiler metadata must not invent a later stable handler, normal completion or native admission when the compiler snapshot is absent.

## Question

Does a genuine immutable chunk compiler selection remain available after current runtime occupancy becomes unknown, while its stable-handler relation stays absent?

## Conclusion

The consumer validates the original head producer and retains the authentic chunk snapshot selection separately from runtime occupancy. Missing current runtime lookup becomes an explicit unknown-before carrier, leaving stable_handler absent. Missing/unknown compiler context still rejects executable admission. This is a Rust implementation contract; native probe results do not execute or prove it.

## Scope

Original full command vector/current head producer and authentic immutable compilation snapshot. Runtime handler lookup, effects, Normal completion, bytecode-object/cache capability and unknown compiler contexts remain independent.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`. Channel: Rust source contract. Version: tcl8.4 not tested for this Rust contract; tcl8.5 not tested for this Rust contract; tcl8.6 not tested for this Rust contract; tcl9.0 not tested for this Rust contract; tcl9.1 not tested for this Rust contract; jim not tested for this Rust contract; bigip not tested for this Rust contract.

No native interpreter executes this compiler-carrier contract.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `select_invocation`: Join current original-head currency with independent chunk compiler selection; unresolved runtime occupancy supplies no stable handler.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `command_binding::compiled_invocation::tests::original_chunk_compiler_selection_survives_unknown_runtime_occupancy` (linked): Across the five authored Tcl compilation dialects, unknown runtime occupancy removes stable_handler while a genuine BytecodeObject chunk retains its original selection; Unknown compilation cannot acquire executable admission. No native or Rust pass is inferred from this binding.

## Evidence

- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs)
