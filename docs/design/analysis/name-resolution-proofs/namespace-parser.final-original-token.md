# naming.namespace-parser.final-original-token

Kind: `native-observation`

## Problem statement

The last parser token inside a variable word can be a TEXT child of the scalar/index, a command/backslash token, or a written trailing suffix. Treating the displayed whole word as a compiler tail loses this topology.

## Question

What numComponents, final Tcl token type and exact final token bytes are reported for the eight original global variable words?

## Conclusion

All five C releases report the same eight parser rows: scalar roots end in TEXT ::name; empty index ends in empty TEXT; qualified/nested index tails end in TEXT ::tail or ::name; command index ends in COMMAND [foo]; escaped index ends in BS \x61; trailing written suffix ends in TEXT ::tail. Equal parser rows do not imply equal release-selected compiler admission.

## Scope

Original Tcl_ParseCommand is called on eight complete ASCII sources without evaluating them. The observer prints raw token type/count/bytes, including empty TEXT. Five C release associations; no variable read, emitted opcode, guest handler result, Jim or BIG-IP inference.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original Tcl_ParseCommand on complete counted ASCII source; no evaluation. Version: tcl8.4 8.4.20 (capture association); tcl8.5 8.5.19 (capture association); tcl8.6 8.6.18 (capture association); tcl9.0 9.0.4 (capture association); tcl9.1 9.1.0 (capture association).

Original case/word-component-count/final-token-type/final-token-hex:

```text
0|2|4|3a3a6e616d65
1|2|4|3a3a6e616d65
2|3|4|
3|3|4|3a3a7461696c
4|4|4|3a3a6e616d65
5|3|16|5b666f6f5d
6|3|8|5c783631
7|5|4|3a3a7461696c
```
Numeric token types are interpreted against the retained probe/public Tcl parser API, not a native runtime read.

### jim, bigip

Status: `not-tested`.

No original Tcl parser final-token capture for this provider is attached.

## Consumer bindings

- [rust/tcl-registry/src/native_namespace_binding_compilation.rs](../../../../rust/tcl-registry/src/native_namespace_binding_compilation.rs), `native_namespace_binding_compilation::tests::namespace_dynamic_tails_follow_original_variable_token_children` (linked): Checks the independent compiler recipe consumes the actual frozen scalar/index/suffix topology and retains release-specific admission; no Native Tcl_ParseCommand execution is asserted.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_final_token/probe.c](../../../../rust/tcl-registry/tests/data/native_namespace_final_token/probe.c)
- [rust/tcl-registry/tests/data/native_namespace_final_token/manifest.json](../../../../rust/tcl-registry/tests/data/native_namespace_final_token/manifest.json)
- [rust/tcl-registry/tests/data/native_namespace_final_token/8.4.20.txt](../../../../rust/tcl-registry/tests/data/native_namespace_final_token/8.4.20.txt)
- [rust/tcl-registry/tests/data/native_namespace_final_token/8.5.19.txt](../../../../rust/tcl-registry/tests/data/native_namespace_final_token/8.5.19.txt)
- [rust/tcl-registry/tests/data/native_namespace_final_token/8.6.18.txt](../../../../rust/tcl-registry/tests/data/native_namespace_final_token/8.6.18.txt)
- [rust/tcl-registry/tests/data/native_namespace_final_token/9.0.4.txt](../../../../rust/tcl-registry/tests/data/native_namespace_final_token/9.0.4.txt)
- [rust/tcl-registry/tests/data/native_namespace_final_token/9.1.0.txt](../../../../rust/tcl-registry/tests/data/native_namespace_final_token/9.1.0.txt)
