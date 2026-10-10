# naming.alias.captured-prefix-allocation-contract

Kind: `implementation-contract`

## Problem statement

An interpreter alias captures argument objects when installed. A later write to the source variable must not replace those captured values, while replacing the alias allocation must replace its capture. Reconstructing the prefix from displayed strings or treating it as a written argument loses its provenance and can donate incorrect compiler-local geometry.

## Question

Does original alias traversal transport captured prefix inputs only with the actual alias allocation at every hop, preserving held values independently of current source variable contents?

## Conclusion

The canonical command table joins each alias hop to the retained current publication and implementation allocation before appending its original captured inputs. SourceCommandTarget retains those inputs separately from effective words, and the shared argument owner marks them BindingPrefix rather than Written. Known later cell writes do not substitute new objects into the old capture; replacement selects the new alias capture. Missing lineage or disagreeing paths refuse the facet. This supplies original effective operand provenance, not writable source words, compiler preparation or normal completion.

## Scope

Current Compiler implementation contract for same-interpreter original alias captures and nested aliases. The regression distinguishes a held prefix across a known scalar write and a replaced alias allocation. Unknown content/observer effects, cross-interpreter aliases, native compilation and actual guest object identity remain independent and are not claimed. No Rust test outcome or native execution is claimed.

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

- `owner-0` (implementation): [rust/tcl-compiler/src/command_binding/original_command_table.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_table.rs). SHA-256 `849a48bae53c0cd389f227b342efd2ca5e9d62d45701e40ad1fe2088580a99c6`. Inspected current shared producer, readonly metadata and regression assertion source; this evidence supplies no native execution outcome.

- `owner-1` (implementation): [rust/tcl-compiler/src/command_binding/original_name_value.rs](../../../../rust/tcl-compiler/src/command_binding/original_name_value.rs). SHA-256 `efac70f728d1d88f2cc83702554b52bb8c27845459e500e080fdf269c5f8f18e`. Inspected current shared producer, readonly metadata and regression assertion source; this evidence supplies no native execution outcome.

- `owner-2` (implementation): [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs). SHA-256 `2959d3909763c4d8e0b627fec9abf5ce5a74449df958cd1450a65264459f5320`. Inspected current shared producer, readonly metadata and regression assertion source; this evidence supplies no native execution outcome.

## Source inspection

No native implementation source excerpt is attached. Native outputs cannot establish this Rust provenance invariant.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/original_command_table.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_table.rs), `original_targets_for_input`: Traverses the actual canonical table and alias allocation while retaining exact captured input lineage.

- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `command_binding::source_command_world::tests::original_alias_prefix_inputs_follow_actual_allocations_and_held_values` (linked): Checks the independently retained original inputs and allocation/provider/receiver-axis boundary stated in this contract.

A named test is a coverage binding, not a claim that it executed.

## Replay

Execute the named selectors under the coherent Rust workspace before claiming a passing result. No native replay is attached to this implementation contract; source changes require evidence reissue.
