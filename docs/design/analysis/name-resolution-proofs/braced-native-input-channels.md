# naming.expression.braced-native-input-channels

Kind: `native-observation`

## Problem statement

Raw counted NUL, C080 bytes, source escapes and file UTF-8 decoding occupy different input stages and can produce different operand bytes even when comparison results match.

## Question

How do counted-native evaluation and the actual C UTF-8/auto ReadChars channel differ for the retained Unicode and zero inputs?

## Conclusion

Raw counted zero remains 00 in the collector; C ReadChars converts file raw zero to C080. C source numeric escapes produce C080 while Jim produces 00; a braced written numeric escape remains literal backslash bytes until expression evaluation. C8.4/8.5 counted astral and ReadChars astral have distinct recorded units/bytes; C8.6 uses surrogate units and C9 scalar units. C9.0/C9.1 ReadChars rejects raw C080 with read count -1 and evaluation is not attempted, while counted C080 evaluates successfully. Jim ReadChars is explicitly not tested. The collector is a separate authored command, not an observation of the original expression handler argv.

## Scope

Exact retained 23 source pairs, C8.4.20/C8.5.19/C8.6.18/C9.0.4/C9.1.0 and Jim 0.84-9-g5bac7c9; finite subquestion cases: string-literal-bmp, string-literal-astral, escaped-native-zero, raw-zero-literal-string, modified-zero-literal-string, escaped-zero-literal-string. Authored argv collector is an independent command. Primary/resident state is sampled before its public byte getter; Unicode getter follows separately. No body/Normal/cache/observer/equivalence/Rust execution grant.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.4 8.4.20.

Raw counted zero remains 00 in the collector; C ReadChars converts file raw zero to C080. C source numeric escapes produce C080 while Jim produces 00; a braced written numeric escape remains literal backslash bytes until expression evaluation. C8.4/8.5 counted astral and ReadChars astral have distinct recorded units/bytes; C8.6 uses surrogate units and C9 scalar units. C9.0/C9.1 ReadChars rejects raw C080 with read count -1 and evaluation is not attempted, while counted C080 evaluates successfully. Jim ReadChars is explicitly not tested. The collector is a separate authored command, not an observation of the original expression handler argv. Tcl 8.4 GetReturnOptions and four-argument catch options are API-unavailable in this probe.

### tcl8.5, tcl8.6

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl8.5 8.5.19; tcl8.6 8.6.18.

Raw counted zero remains 00 in the collector; C ReadChars converts file raw zero to C080. C source numeric escapes produce C080 while Jim produces 00; a braced written numeric escape remains literal backslash bytes until expression evaluation. C8.4/8.5 counted astral and ReadChars astral have distinct recorded units/bytes; C8.6 uses surrogate units and C9 scalar units. C9.0/C9.1 ReadChars rejects raw C080 with read count -1 and evaluation is not attempted, while counted C080 evaluates successfully. Jim ReadChars is explicitly not tested. The collector is a separate authored command, not an observation of the original expression handler argv.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Counted Tcl_EvalEx source; separate Tcl_ReadChars on file UTF-8/auto then counted Tcl_EvalEx; independent authored collector and separate object-vector catch context. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Raw counted zero remains 00 in the collector; C ReadChars converts file raw zero to C080. C source numeric escapes produce C080 while Jim produces 00; a braced written numeric escape remains literal backslash bytes until expression evaluation. C8.4/8.5 counted astral and ReadChars astral have distinct recorded units/bytes; C8.6 uses surrogate units and C9 scalar units. C9.0/C9.1 ReadChars rejects raw C080 with read count -1 and evaluation is not attempted, while counted C080 evaluates successfully. Jim ReadChars is explicitly not tested. The collector is a separate authored command, not an observation of the original expression handler argv. C080 DocumentReadChars original/candidate records read=-1, empty result and exact queried error options; direct evaluation not attempted for those two inputs.

### jim

Status: `observed`. Channel: Counted Jim_NewStringObj/Jim_EvalObj; independent authored argv collector; separate Jim_EvalObjVector catch context. Version: jim 0.84-9-g5bac7c9.

Raw counted zero remains 00 in the collector; C ReadChars converts file raw zero to C080. C source numeric escapes produce C080 while Jim produces 00; a braced written numeric escape remains literal backslash bytes until expression evaluation. C8.4/8.5 counted astral and ReadChars astral have distinct recorded units/bytes; C8.6 uses surrogate units and C9 scalar units. C9.0/C9.1 ReadChars rejects raw C080 with read count -1 and evaluation is not attempted, while counted C080 evaluates successfully. Jim ReadChars is explicitly not tested. The collector is a separate authored command, not an observation of the original expression handler argv. Jim has counted Jim_EvalObj and separate object-vector catch only; C ReadChars is not tested and trace is guest unavailable.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression](../../../../rust/tcl-compiler/tests/data/native_brace_expression)
- [rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v1/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v3/protocol)
- [rust/tcl-compiler/tests/data/native_brace_expression/v4/capture](../../../../rust/tcl-compiler/tests/data/native_brace_expression/v4/capture)
