# naming.refactor.variable-inline-unbraced-unicode-read

Kind: `native-observation`

## Problem statement

Removing a store based on a printed Unicode name can change a source that the C evaluator parses as a shorter variable reference.

## Question

How does bare $café differ from braced ${café} in the retained pure source controls?

## Conclusion

C8.4 through C9.1 parse bare $café as the read of caf followed by literal é, and the file-script process exits1 with the missing caf error. Braced ${café} returns 7. Jim reads the full bare name and returns 7. The separate stdin capture preserves the C error in stderr even though its interactive shell process exits0; status0 alone does not establish a successful read.

## Scope

Fixed independently authored UTF-8 CLI script controls in a fresh native process. The source uses ordinary textual names and braced or unbraced variable syntax; it contains no raw native name-object, counted EvalObjv, physical header or byte-table observation. Handwritten naive scripts are controls, not captured output from the Rust refactor. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: tcl8.4 8.4.20.

file pure_unicode_bare_original: exit 1, stdout '', stderr 'can\'t read "caf": no such variable\n    while executing\n"puts $café"\n    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c84-pure_unicode_bare_original.tcl" line 2)\n'. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. stdin pure_original: exit 0, stdout '', stderr 'can\'t read "caf": no such variable\n'. stdin pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl8.5

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: tcl8.5 8.5.19.

file pure_unicode_bare_original: exit 1, stdout '', stderr 'can\'t read "caf": no such variable\n    while executing\n"puts $café"\n    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c85-pure_unicode_bare_original.tcl" line 2)\n'. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. stdin pure_original: exit 0, stdout '', stderr 'can\'t read "caf": no such variable\n'. stdin pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl8.6

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: tcl8.6 8.6.18.

file pure_unicode_bare_original: exit 1, stdout '', stderr 'can\'t read "caf": no such variable\n    while executing\n"puts $café"\n    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c86-pure_unicode_bare_original.tcl" line 2)\n'. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. stdin pure_original: exit 0, stdout '', stderr 'can\'t read "caf": no such variable\n'. stdin pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl9.0

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: tcl9.0 9.0.4.

file pure_unicode_bare_original: exit 1, stdout '', stderr 'can\'t read "caf": no such variable\n    while executing\n"puts $café"\n    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c90-pure_unicode_bare_original.tcl" line 2)\n'. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. stdin pure_original: exit 0, stdout '', stderr 'can\'t read "caf": no such variable\n'. stdin pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### tcl9.1

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: tcl9.1 9.1.0.

file pure_unicode_bare_original: exit 1, stdout '', stderr 'can\'t read "caf": no such variable\n    while executing\n"puts $café"\n    (file "/workspace/.proofs/captured-inline-variable-semantics-v2/c91-pure_unicode_bare_original.tcl" line 2)\n'. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. stdin pure_original: exit 0, stdout '', stderr 'can\'t read "caf": no such variable\n'. stdin pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### jim

Status: `observed`. Channel: UTF-8 file script passed as the public shell argv filename; UTF-8 stdin source passed to the interactive public shell; guest errors can coexist with process0. Version: jim 0.84-9-g5bac7c9.

file pure_unicode_bare_original: exit 0, stdout '7\n', stderr ''. file pure_unicode_braced_original: exit 0, stdout '7\n', stderr ''. stdin pure_original: exit 0, stdout '7\n', stderr ''. stdin pure_inlined: exit 0, stdout '7\n', stderr ''. The receipt retains the executed version output, exact shell SHA, source SHA, exit and both raw streams. Native configure flags, linked-library attribution and compiler version are not recorded by this shell experiment; no independently captured build identity is borrowed.

### bigip

Status: `not-tested`.

No BIG-IP appliance result is attached for this precise native source question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/file)
- [rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/stdin](../../../../rust/tcl-lsp-core/tests/data/native_inline_variable_semantics/stdin)
