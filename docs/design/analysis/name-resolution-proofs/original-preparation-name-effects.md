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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::original_argument_name_effects_in_context`: Retain the original selected preparation visits independently of opcode admission.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompiledBodyContext::entered_context`: Preserve selected loop and exception compiler environments without inventing entry mode or nesting.
- [rust/tcl-registry/src/native_expression_program.rs](../../../../rust/tcl-registry/src/native_expression_program.rs), `NativeExpressionProgram::original_command_substitutions`: Read nested command topology from the retained checked expression tree and source.
- [rust/tcl-compiler/src/command_binding/original_compiler_effects.rs](../../../../rust/tcl-compiler/src/command_binding/original_compiler_effects.rs), `preserves_visits`: Close exact original nested compiler effects against the immutable current registration and independently selected resolver world.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_compiler_effect_visits_preserve_native_control_pruning` (linked): Original conditional compiler plans retain reached branches and prune constant-false bodies while opcode selection remains Unknown; an invalid original ordinal remains unavailable.
- [rust/tcl-compiler/src/command_binding/compiled_invocation.rs](../../../../rust/tcl-compiler/src/command_binding/compiled_invocation.rs), `command_binding::compiled_invocation::tests::original_nested_compiler_effects_preserve_names_without_executing_children` (linked): Nested data-only, expression-bracket, quoted-expression and array-index compiler visits close independently of child runtime mutation; unresolved expression functions withdraw and every outer admission remains Unknown with no native preparation or proofs.

## Evidence

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs)
- [rust/tcl-registry/src/native_expression_program.rs](../../../../rust/tcl-registry/src/native_expression_program.rs)
- [rust/tcl-compiler/src/command_binding/original_compiler_effects.rs](../../../../rust/tcl-compiler/src/command_binding/original_compiler_effects.rs)
