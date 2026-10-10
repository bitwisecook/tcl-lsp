# naming.compiler.original-preparation-name-effects

Kind: `implementation-contract`

## Problem statement

A selected stock compiler can have Unknown opcode admission while recursively visiting original bracket substitutions, conditional bodies or expressions. Treating that uncertainty as an arbitrary command-table mutation loses later source declarations; treating a literal outer command or handler footprint as proof of quiet nested compilation would hide a custom compiler or resolver callback. The exact compiler visits and current lookup world must be retained independently of runtime execution.

## Question

How can original nested compiler preparation preserve tracked command and namespace names while leaving native opcode admission, runtime child execution and handler completion unresolved?

## Conclusion

The Registry projects preparation visits from its existing original control and expression owners. The Compiler visits exact unchanged original body extents and lexical word substitutions in one immutable compiler table, verifies each selected registration and dependency, and requires independently closed command-resolver lookup. Unknown functions, custom hooks, malformed scripts, changed body mapping and missing resolver evidence withdraw the effect closure. Conditional pruning and selected loop/catch contexts remain owned by the original control plan. No command executes and no CPP, Normal, physical-object or edit receipt is issued.

## Scope

Implementation contract for selected C Tcl 8.4–9.1 original source preparation. Exact image/channel/full LexerConfig and selected native source-string policy are retained. Jim and BIG-IP preparation are outside this issuer. Expression functions remain unresolved and require a separate function/resolver owner. Embedded command substitutions in a checked expression retain exact original bracket-body extents and use the same compiler-only visit queue; quoted and array-index commands retain their own checked geometry. Parser-expanded or transformed script bodies require a separate mapping and decline here.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `implementation-0` (implementation): [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs). SHA-256 `0e02a7271419386b37ad455ccb0dab9c575f58133998972dc339528c112e7f4e`. Retain the original selected preparation visits independently of opcode admission.
- `implementation-1` (implementation): [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs). SHA-256 `0e02a7271419386b37ad455ccb0dab9c575f58133998972dc339528c112e7f4e`. Preserve selected loop and exception compiler environments without inventing entry mode or nesting.
- `implementation-2` (implementation): [rust/tcl-registry/src/native_expression_program.rs](../../../../rust/tcl-registry/src/native_expression_program.rs). SHA-256 `7981bf64f1361b4187e841105f2296cdd0eb76b1207eeadd9d846ea9c8303cfb`. Read nested command topology from the retained checked expression tree and source.
- `implementation-3` (implementation): [rust/tcl-compiler/src/command_binding/original_compiler_effects.rs](../../../../rust/tcl-compiler/src/command_binding/original_compiler_effects.rs). SHA-256 `fade48450e4ce0fbfa8b6e1839b570d0268b7d03ee06a6623d9328323c9fc20a`. Close exact original nested compiler effects against the immutable current registration and independently selected resolver world.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::original_argument_name_effects_in_context`: Retain the original selected preparation visits independently of opcode admission.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompiledBodyContext::entered_context`: Preserve selected loop and exception compiler environments without inventing entry mode or nesting.
- [rust/tcl-registry/src/native_expression_program.rs](../../../../rust/tcl-registry/src/native_expression_program.rs), `NativeExpressionProgram::original_command_substitutions`: Read nested command topology from the retained checked expression tree and source.
- [rust/tcl-compiler/src/command_binding/original_compiler_effects.rs](../../../../rust/tcl-compiler/src/command_binding/original_compiler_effects.rs), `preserves_visits`: Close exact original nested compiler effects against the immutable current registration and independently selected resolver world.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_compiler_effect_visits_preserve_native_control_pruning` (linked): Original conditional compiler plans retain reached branches and prune constant-false bodies while opcode selection remains Unknown; an invalid original ordinal remains unavailable.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `command_binding::compiled_invocation::tests::original_nested_compiler_effects_preserve_names_without_executing_children` (linked): Nested data-only, expression-bracket, quoted-expression and array-index compiler visits close independently of child runtime mutation; unresolved expression functions withdraw and every outer admission remains Unknown with no native preparation or proofs.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-compiler",
  "original_nested_compiler_effects_preserve_names_without_executing_children"
]
```

Run the listed Registry selector against the complete current implementation. Coverage bindings describe assertions; no native provider or Rust execution result is attached to this record.
