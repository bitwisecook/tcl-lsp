# naming.tcloo.public-vs-native-object-command-client-data

Kind: `native-observation`

## Problem statement

The legacy public Tcl_CmdInfo record is distinct from the actual current native Command ABI.

## Question

Do the legacy public clientData and selected native Command clientData fields denote the same actual TclOO Object?

## Conclusion

The legacy public Tcl_GetCommandInfoFromToken clientData matches the observed object in C8.6 and differs in C9.0/C9.1. Reading the configured actual native Command field matches the object in all three: objClientData in C8.6/C9.0, objClientData2 in C9.1. V2 incorrectly selected the C9.1 field for C9.0 and failed compilation before native execution; V3 corrects that release selection without rewriting the attempt.

## Scope

Configured original C private/public object APIs on fresh bootstrap and explicitly printed rename/lifecycle/dispatch controls. ASCII Tcl_EvalEx source controls and actual selected header fields; no compiler admission, cache, variable, method-body, observer, Registry-role or Rust parity capability. C8.4/C8.5/Jim expose no oo::class in the measured initialized interpreter. BIG-IP not tested.

## Provider answers

### tcl8.4, tcl8.5

Status: `unsupported`. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Configured direct C object/private-header APIs plus exact ASCII Tcl_EvalEx source controls. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

The legacy public Tcl_GetCommandInfoFromToken clientData matches the observed object in C8.6 and differs in C9.0/C9.1. Reading the configured actual native Command field matches the object in all three: objClientData in C8.6/C9.0, objClientData2 in C9.1. V2 incorrectly selected the C9.1 field for C9.0 and failed compilation before native execution; V3 corrects that release selection without rewriting the attempt.

### jim

Status: `unsupported`. Channel: Exact ASCII Jim_Eval info patchlevel/info commands; no TclOO private-header API. Version: jim 0.84-9-g5bac7c9.

Measured info commands ::oo::class is empty; bootstrap field API question has no object to inspect. Probe availability sentinel is not an object observation.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v1)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v3)
- [rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/compile.stderr](../../../../rust/tcl-vm/tests/data/native_oo_bootstrap_capability/v2/9.0.4/compile.stderr)
