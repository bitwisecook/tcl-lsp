# naming.variable.catch-original-output-name

Kind: `native-observation`

## Problem statement

A catch output name can be opaque native bytes even when its display is invalid or contains zero. Reconstructing it from a display can write the replacement-character name or clip a counted operand. The output result, plain-name sentinel and replacement-character sentinel must be checked separately, and the old C8.4 setter extent cannot be generalized to later engines.

## Question

Where does public catch store BOOM when its original output name contains ff, binary zero or encoded C080, and do plain or replacement-character sibling cells change?

## Conclusion

All six providers return catch status 1 and reading the exact original output name returns BOOM. The replacement-character sibling remains REPLACEMENT. Raw ff and C080 preserve the plain r sentinel on all six. With binary zero, only C8.4 changes plain r to BOOM; C8.5/C8.6/C9.0/C9.1/Jim keep plain r as PLAIN. These are direct output-setter/result observations, not alias, trace, normal-completion, header/cache or complete namespace laws.

## Scope

Each fresh interpreter starts with r=PLAIN and r plus U+FFFD UTF8 bytes=REPLACEMENT. Public catch receives original counted r+ff, r+00+tail or r+C080+tail and ASCII error BOOM body. Each result is inspected before the next command replaces it. No optionVarName, callbacks, aliases, array names or missing namespace is tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source. Version: tcl8.4 8.4.20.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output also changes plain r to BOOM. Raw ff and C080 keep the plain sentinel.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### jim

Status: `observed`. Channel: Original counted string objects passed directly to public Jim_EvalObjVector; ASCII Jim_Eval setup/version source. Version: jim 0.84-9-g5bac7c9.

All exact output reads BOOM; replacement sibling unchanged. Binary-zero output leaves plain r as PLAIN. Raw ff and C080 keep the plain sentinel.

### bigip

Status: `not-tested`.

No appliance observation for these exact inputs.

## Consumer bindings

- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `finish_catch`: Store through the original result-variable object using the selected native setter purpose.
- [rust/tcl-vm/src/command/native_subst_operand_tests.rs](../../../../rust/tcl-vm/src/command/native_subst_operand_tests.rs), `command::native_subst_operand_tests::catch_original_output_names_match_counted_native_controls` (linked): Compare exact catch result codes and counted output-name reads, plain/replacement sibling values and native result bytes for the three independent original byte inputs. No option table, alias, observer, normal completion or Rust execution is inferred.
- [runtime/rust/src/builtins/native_subst_operand_tests.rs](../../../../runtime/rust/src/builtins/native_subst_operand_tests.rs), `builtins::native_subst_operand_tests::catch_original_output_names_match_counted_native_controls` (linked): Compare exact catch result codes and counted output-name reads, plain/replacement sibling values and native result bytes for the three independent original byte inputs. No option table, alias, observer, normal completion or Rust execution is inferred.

## Evidence

- [rust/tcl-vm/tests/data/native_subst_catch_operands/v1](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1)
- [rust/tcl-vm/tests/data/native_subst_catch_operands/v2](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2)
