# naming.substitution.original-options-and-flag-families

Kind: `native-observation`

## Problem statement

An original option object may contain binary zero, encoded zero or opaque bytes. A display conversion or shared full-count matcher can change its selected option and diagnostic. Tcl 9.1 also adds positive flag options whose defaults differ from negative options. Correct template flags require the actual option declaration and original getter/cache purpose; the following controls measure result code and bytes only.

## Question

How do direct counted original Subst option objects distinguish binary zero from C080/ff, and which engines accept positive flag options or mixed families?

## Conclusion

All six linked engines accept -novariables followed by binary zero and leave $x literal. Encoded C080 and raw ff suffixes fail with their original diagnostic bytes. Only C9.1 accepts the three positive options: -backslashes and -commands leave $x literal, while -variables returns VALUE. C9.1 rejects -variables combined with -nocommands. C8.4/C8.5 use the diagnostic noun switch; C8.6/C9/Jim use option. Native result bytes do not prove option-cache/header effects or Rust implementation parity.

## Scope

Original counted option objects through Tcl_NewStringObj/Tcl_EvalObjv or Jim_NewStringObj/Jim_EvalObjVector; template $x and ASCII setup x=VALUE. Subst v1 has three option controls; v2 additionally has positive and mixed controls. No source numeric escapes, document ingress, internal-representation observation, errorCode or arbitrary template behavior is claimed.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. Positive options rejected; three-entry diagnostic with noun switch.

### tcl8.6, tcl9.0

Status: `observed`. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source. Version: tcl8.6 8.6.18; tcl9.0 9.0.4.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. Positive options rejected; three-entry diagnostic with noun option.

### tcl9.1

Status: `observed`. Channel: Original counted string objects passed directly to public Tcl_EvalObjv; ASCII Tcl_Eval setup/version source. Version: tcl9.1 9.1.0.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. All three positive options accepted; mixed families rejected; six-entry option diagnostic.

### jim

Status: `observed`. Channel: Original counted string objects passed directly to public Jim_EvalObjVector; ASCII Jim_Eval setup/version source. Version: jim 0.84-9-g5bac7c9.

Binary-zero suffix matches; C080 and ff suffixes fail with exact diagnostic bytes. Positive options rejected; three-entry diagnostic with noun option.

### bigip

Status: `not-tested`.

No appliance observation for these exact inputs.

## Consumer bindings

- [rust/tcl-registry/src/substitution.rs](../../../../rust/tcl-registry/src/substitution.rs), `NativeSubstitutionOptions`: Select the actual option table and fold canonical flag families; no object-cache or handler registration grant.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `cmd_subst`: Consume original option objects then retain selected template flags.
- [runtime/rust/src/builtins.rs](../../../../runtime/rust/src/builtins.rs), `subst_cmd`: Consume original option objects through the independent selected static index/enum owner.
- [rust/tcl-vm/src/command/native_subst_operand_tests.rs](../../../../rust/tcl-vm/src/command/native_subst_operand_tests.rs), `command::native_subst_operand_tests::original_subst_options_match_counted_native_families` (linked): Compare exact captured result code and native result bytes for original byte inputs; pure Registry test independently checks available authored table parity/flag folding. No native observation alone asserts Rust execution.
- [runtime/rust/src/builtins/native_subst_operand_tests.rs](../../../../runtime/rust/src/builtins/native_subst_operand_tests.rs), `builtins::native_subst_operand_tests::original_subst_options_match_counted_native_families` (linked): Compare exact captured result code and native result bytes for original byte inputs; pure Registry test independently checks available authored table parity/flag folding. No native observation alone asserts Rust execution.
- [rust/tcl-registry/src/substitution.rs](../../../../rust/tcl-registry/src/substitution.rs), `substitution::tests::native_runtime_option_declaration_matches_registry_surface_and_family_fold` (linked): Compare exact captured result code and native result bytes for original byte inputs; pure Registry test independently checks available authored table parity/flag folding. No native observation alone asserts Rust execution.

## Evidence

- [rust/tcl-vm/tests/data/native_subst_catch_operands/v1](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v1)
- [rust/tcl-vm/tests/data/native_subst_catch_operands/v2](../../../../rust/tcl-vm/tests/data/native_subst_catch_operands/v2)
