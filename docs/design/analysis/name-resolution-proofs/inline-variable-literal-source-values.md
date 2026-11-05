# naming.refactor.variable-inline-literal-source-values

Kind: `native-observation`

## Problem statement

A literal-variable rewrite needs the same actually parsed read and literal output; equal reporting labels do not establish that source relationship.

## Question

Do the pure ASCII and braced Unicode original reads return the same value as the exact literal replacement control?

## Conclusion

All six measured shells return 7 with process0 and empty stderr for ASCII $x, braced ${café}, and the literal puts 7 control. These finite pure scripts establish value equality for those exact sources, independently of a rewrite permission for other cells, values or observers.

## Scope

Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

file pure_ascii_original: exit 0, stdout '7\n', stderr ''. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. file pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### bigip

Status: `not-tested`.

No BIG-IP appliance result is attached for this precise native source question.

## Consumer bindings

- [rust/tcl-lsp-core/src/refactor/inline_variable.rs](../../../../rust/tcl-lsp-core/src/refactor/inline_variable.rs), `refactor::inline_variable::tests::original_inline_variable_requires_exact_definition_and_unobserved_read_without_ui_maps` (linked): Select the exact braced Unicode read and original literal store without reporting variables; refuse stale source. This source consumer assertion is independent of the recorded native shell outcomes.

## Evidence

- [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file)
- [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/replay.py](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/replay.py)
