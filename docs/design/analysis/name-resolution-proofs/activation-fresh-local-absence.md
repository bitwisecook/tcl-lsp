# naming.activation.fresh-local-absence

Kind: `native-observation`

## Problem statement

A root variable b is already PRESENT when f starts. An existence consumer could accidentally borrow that namespace cell for an unlinked procedure local b, or invent an array element Params(key) that was never created in the activation.

## Question

Does a fresh procedure activation report either unlinked b or never-created Params(key) present after root b was set?

## Conclusion

All six selected shell captures return 0 0, with process status0 and empty stderr. Root b does not make either queried procedure-local receiver present in this exact unlinked activation. This finite source observation does not establish arbitrary incoming frame state, compiler-local allocation, observers or an entered native variable object.

## Scope

One ASCII script sets ::b, defines zero-argument f, invokes it and prints two info-exists values. The exact source hash, selected executable hash, process status and embedded streams are retained. Full patchlevel/configuration and original shell channel are unrecorded. No raw NUL, opaque bytes, linked receiver, TclOO or BIG-IP run occurs.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: ASCII shell source; original file-versus-stdin argument vector unrecorded. Version: jim Jim0.84.

Actual output is 0 0: unlinked local b and never-created Params(key) are both absent. Process status0 and empty stderr are retained independently of the two Boolean results.

### bigip

Status: `not-tested`.

No appliance capture for this exact program is attached; C Tcl and Jim outcomes do not establish BIG-IP load or event behaviour.

## Consumer bindings

- [rust/tcl-compiler/src/compiler_checks.rs](../../../../rust/tcl-compiler/src/compiler_checks.rs), `function_nontaint_checks`: Consume the function's retained diagnostic constant-branch facts while excluding Selected branch facts; this source diagnostic query does not issue Native frame existence or absence authority.
- [rust/tcl-compiler/src/analyser/diagnostics/tests.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/tests.rs), `analyser::diagnostics::tests::info_exists_fresh_activation_absence_does_not_borrow_namespace_contents` (linked): Binds the exact native source and checks I230 for the b predicate in an independently analysed source. This Rust assertion does not reproduce the Params element query or manufacture a native frame.

## Evidence

- [rust/tcl-compiler/tests/data/native_activation_presence/source.tcl](../../../../rust/tcl-compiler/tests/data/native_activation_presence/source.tcl)
- [rust/tcl-compiler/tests/data/native_activation_presence/manifest.json](../../../../rust/tcl-compiler/tests/data/native_activation_presence/manifest.json)
