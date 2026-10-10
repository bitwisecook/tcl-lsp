# naming.compiler.original-compiler-operand-coordinates

Kind: `implementation-contract`

## Problem statement

Selected Runtime String-member metadata starts after its selector, while the original public compiler vector retains that selector. Reusing the Runtime argument offset can erase a genuine compiler word or fabricate a private worker operand.

## Question

How does selected native-compilation metadata project its compiler operands onto the unchanged original argv while retaining a public String selector and refusing an absent private operand?

## Conclusion

NativeCompilationSpec::original_operand_from_for_facts consumes already selected InvocationFacts and delegates the selected implementation path. Public String equal/length/trim grammar keeps the original member selector as a compiler operand; private worker grammar starts at that worker’s own first operand. original_registration_descriptor and native_compilation_syntax use this shared coordinate owner with the genuine complete original vector. Missing selected member shape refuses rather than reconstructing a head or member. The source replacement control distinguishes selected original compiler shapes from generic argv relookup when the original argument produces a replacement of string; it retains each version’s selected authored horizon without asserting actual runtime execution. Independent original native scalar windows establish their own measured shapes, not this source-model replacement outcome.

## Scope

Two fixed source/API controls compare selected metadata with complete original public/private compiler coordinates and version-selected original replacement horizons. Their values do not execute an external C/Jim/BIG-IP provider, observe private operands/objects or grant current native binding, entered frame, successful compilation/runtime completion or effects. All seven native providers are not tested for this implementation question.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-compiler-original-compiler-operand-coordinates-compiled_invocation.rs` (implementation): [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs). SHA-256 `6007bf9693c2e709eed7f8443043893c4687b12e2a88a4f149b25edccdbb11db`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-compiler-original-compiler-operand-coordinates-registry_invocation.rs` (implementation): [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs). SHA-256 `a52f77eae9752f9a020f7a1f952c747b4856939396be1edcff5e5fc0fe3e0a4a`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-compiler-original-compiler-operand-coordinates-native_compilation.rs` (implementation): [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs). SHA-256 `9f471133efc3f808a23eb55f09afa277b616960bad64bbbb3a3e5054a07dac6d`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-compiler-original-compiler-operand-coordinates-native_scalar_compilation.rs` (implementation): [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs). SHA-256 `aa1523a6d5e03dd01384ba5743a5f8756e8170db1ab40002edc4f48e6b48c253`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::original_operand_from_for_facts`: Project already selected compiler grammar onto the complete unchanged original vector independently of Runtime member offset.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `original_registration_descriptor`: Require selected original registration grammar and its genuine compiler vector coordinates; no decoded member reconstruction.
- [rust/tcl-compiler/src/registry_invocation.rs](../../../../rust/tcl-compiler/src/registry_invocation.rs), `native_compilation_syntax`: Retain compiler operand geometry separately from Runtime selected member metadata and invocation admission.
- [rust/tcl-registry/src/native_scalar_compilation.rs](../../../../rust/tcl-registry/src/native_scalar_compilation.rs), `native_scalar_compilation::tests::original_scalar_coordinates_retain_the_selected_public_member` (linked): Complete genuine public String vectors retain the original member selector, while private worker grammar uses its own first operand; missing selected member shape refuses. Native scalar-window observations are independent of this source-coordinate projection.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `command_binding::compiled_invocation::tests::string_equal_public_replacement_follows_its_original_compiler_shape` (linked): Across all selected C authored recipes, original string equal/abbreviation/quoted/braced/options shapes retain selected compiler versus generic replacement horizons. This source-model target distinction executes no native process and gives no successful runtime grant.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
