# naming.property.opaque-name-custom-getter-option-cache

Kind: `native-observation`

## Problem statement

An original property name may include invalid FF or counted zero/tail bytes. Passing the full counted management operand can succeed without installing a cache on that caller object. Treating success as proof of a value-type getter or a distinct post-zero property identity would exceed this probe.

## Question

Can the two counted original names xFF and x00tail select a custom property getter while leaving the caller management-option primary absent?

## Conclusion

C9.1 completes both original object-vector definitions/configure calls and prints opaque-0 RAW none and opaque-1 RAW none. The custom getter body is return RAW, and the observed caller option has no primary at the sampled point. This does not measure a custom opaque value ObjType, value getter callbacks, distinction between x and x00tail declarations, or absence of a cache on the independently selected original enumeration member.

## Scope

Two counted Tcl_NewStringObj property names: xFF length2 and x00tail length6; dashed caller options are length3/7. An oo::configurable class and object are created through source setup, then properties and management reads run through public original-object Tcl_EvalObjv flags0. Only C9.1 is run; other C releases, Jim and BIG-IP are not-tested for this exact program. Full original probe/status/stdout and capture-time source/build hashes are retained; the exact Runtime/VM fixture copies are one capture, not independent confirmations.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This exact original-object C9.1 probe has no observation for this provider.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This exact original-object C9.1 probe has no observation for this provider.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This exact original-object C9.1 probe has no observation for this provider.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This exact original-object C9.1 probe has no observation for this provider.

### tcl9.1

Status: `observed`. Version: 9.1.0 (original build/source association; probe does not query runtime patchlevel). Build: Exact original source/header/library/executable SHA-256 map in receipt; compiler command/status0 retained, compiler version not recorded.. Channel: compiled public Tcl_EvalObjv original-object argv and entered Tcl_Eval script; private object/cache observer where stated. Dialect: tcl9.1.

Original full rows are opaque-0 RAW none and opaque-1 RAW none. Compile/process status0, empty stderr, no timeout; original TCL_LIBRARY is recorded. Each helper fails the process if an original definition/configure invocation is not code0.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This exact original-object C9.1 probe has no observation for this provider.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This exact original-object C9.1 probe has no observation for this provider.

## Exact evidence

- `probe` (input): [runtime/rust/tests/data/native_property_opaque/probe.c](../../../../runtime/rust/tests/data/native_property_opaque/probe.c). SHA-256 `04294ffd4e1b4ba00693f23b1e570260b16587ba62d36c5d1bc3a172396c4679`. Exact original-object constructors, invocation and pre-render cache observer.
- `receipt` (provider): [runtime/rust/tests/data/native_property_opaque/manifest.json](../../../../runtime/rust/tests/data/native_property_opaque/manifest.json). SHA-256 `f3440403a0c3fae844c59052acc5d5754fffdbfa409f228b70244b3db21456de`. Original compile/process status, original source/header/library/executable digests, environment and full streams.
- `rows` (observation): [runtime/rust/tests/data/native_property_opaque/native.tsv](../../../../runtime/rust/tests/data/native_property_opaque/native.tsv). SHA-256 `5346337dee62ff9f9af8f5bdf5e0cf376435cb0e033346afaf0848728f9cd004`. Exact full original stdout retained separately.
- `capture` (limitation): [runtime/rust/tests/data/native_property_opaque/capture.py](../../../../runtime/rust/tests/data/native_property_opaque/capture.py). SHA-256 `dcd2625f990e870186919efc794031323dcfdacfca83fdddeb182925376e0934`. Original fixed-path capture adapter; it overwrites local outputs and is not a read-only verifier.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `configure_native`: Consumes the original counted caller management option and independently selected property member under the native property lookup recipe.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `configure_native`: Consumes the original counted caller management option and independently selected property member under the native property lookup recipe.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `tests::opaque_property_lookup_uses_original_accessor_members` (linked): Recreates the exact counted xFF/x00tail property/caller management inputs and compares RAW plus absent caller cache. The separate selected-member cache assertion is an implementation obligation, not sampled by this native probe.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `tests::opaque_property_lookup_uses_original_accessor_members` (linked): Recreates the exact counted xFF/x00tail property/caller management inputs and compares RAW plus absent caller cache. The separate selected-member cache assertion is an implementation obligation, not sampled by this native probe.

A named test is a coverage binding, not a claim that it executed.

## Replay

The retained capture.py requires its fixed /workspace native paths and writes new outputs into the fixture directory. It is not run by this record. A fresh replay must independently select the release headers/library/source hashes, preserve the original observation order, and retain the new source/status/streams separately; no executed Rust parity follows from the captured native rows.
