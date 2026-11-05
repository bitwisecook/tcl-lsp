# naming.compiler.literal-emitter-name-effects

Kind: `implementation-contract`

## Problem statement

A source command can have a genuine stock compiler registration and complete original operands while its actual bytecode context remains unknown. The existing source walker treated Unknown opcode admission as an arbitrary command or namespace mutation, even when the selected compiler only prepares literal data and emits an instruction. Equally, literal outer words do not close a compiler that recursively compiles a body or expression, or prepares a fresh command-name object through a resolver-mediated getter. Those competing paths must be distinguished before the actual handler and argv are evaluated.

## Question

How does the shared compiler effect projection preserve tracked names for original literal emitters while retaining unresolved native compilation and handler obligations?

## Conclusion

A shared grammar-family table requires the selected C source-string recipe, complete original word capture and substitution-free full vector. It accepts represented direct-emitter paths, narrowly selects namespace/string branches, and leaves body/expression/template, resolver-mediated named invocation, unauthored hooks and Array paths unavailable. Exact implementation dependencies are met by the source caller. Unknown opcode admission, missing preparations, variable storage and handler normal completion remain unchanged; only the before-argv tracked-name barrier consumes this fact.

## Scope

Rust tracked-name effect contract only. Native source explanation is separate; no Rust or native execution is claimed by these linked selectors. The caller must retain exact current compiler registration and same-owner dependencies, and must close argv, handler, body and normal completion independently.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::preserves_names_before_original_arguments`: Select the independently supported tracked-name effect branch from genuine compiler metadata and complete original literal words.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::preserves_names_for_literal_vector`: Classify represented direct emitter branches; recursive-source and resolver-mediated branches decline without operation or handler footprint inference.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_literal_emitter_families_preserve_names_without_native_admission` (linked): Selected literal emitter metadata is tested separately from Unknown opcode admission and original word ownership; this is no native execution claim.
