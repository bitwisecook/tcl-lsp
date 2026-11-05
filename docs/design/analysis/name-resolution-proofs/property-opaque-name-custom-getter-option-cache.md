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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim, bigip

Status: `not-tested`.

This exact original-object C9.1 probe has no observation for this provider.

### tcl9.1

Status: `observed`. Channel: compiled public Tcl_EvalObjv original-object argv and entered Tcl_Eval script; private object/cache observer where stated. Version: tcl9.1 9.1.0 (original build/source association; probe does not query runtime patchlevel).

Original full rows are opaque-0 RAW none and opaque-1 RAW none. Compile/process status0, empty stderr, no timeout; original TCL_LIBRARY is recorded. Each helper fails the process if an original definition/configure invocation is not code0.

## Consumer bindings

- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `configure_native`: Consumes the original counted caller management option and independently selected property member under the native property lookup recipe.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `configure_native`: Consumes the original counted caller management option and independently selected property member under the native property lookup recipe.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `tests::opaque_property_lookup_uses_original_accessor_members` (linked): Recreates the exact counted xFF/x00tail property/caller management inputs and compares RAW plus absent caller cache. The separate selected-member cache assertion is an implementation obligation, not sampled by this native probe.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `tests::opaque_property_lookup_uses_original_accessor_members` (linked): Recreates the exact counted xFF/x00tail property/caller management inputs and compares RAW plus absent caller cache. The separate selected-member cache assertion is an implementation obligation, not sampled by this native probe.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `cmd_oo::native_properties::tests::opaque_property_lookup_uses_original_accessor_members` (linked): Opaque property lookup uses original accessor members. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `cmd_oo::native_properties::tests::opaque_property_lookup_uses_original_accessor_members` (linked): Opaque property lookup uses original accessor members. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.

## Evidence

- [runtime/rust/tests/data/native_property_opaque/probe.c](../../../../runtime/rust/tests/data/native_property_opaque/probe.c)
- [runtime/rust/tests/data/native_property_opaque/manifest.json](../../../../runtime/rust/tests/data/native_property_opaque/manifest.json)
- [runtime/rust/tests/data/native_property_opaque/native.tsv](../../../../runtime/rust/tests/data/native_property_opaque/native.tsv)
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs)
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs)
