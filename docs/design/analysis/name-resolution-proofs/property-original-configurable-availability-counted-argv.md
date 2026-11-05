# naming.property.original-configurable-availability-counted-argv

Kind: `native-observation`

## Problem statement

Unavailable configurable commands fail the VM’s Native source preflight before returning a guest result. That source refusal cannot be compared to native original-argv invocation by borrowing source-script creation rows.

## Question

What are the exact configurable inventory/factory/class-create results through the six providers’ original counted object-vector door?

## Conclusion

C84/C85/C86 and Jim return an empty configurable inventory and code1 unavailable-command bytes at the fresh factory vector. C90/C91 return the configurable command, create ::C and then ::o with code0. These measured original-argv outcomes remain separate from source compilation/admission and property storage.

## Scope

One fresh interpreter per provider; only the complete literal ASCII vectors info commands ::oo::configurable, oo::configurable create C, and C create o after success. Twenty complete rows include six separately queried actual versions. No opaque name, property storage, cached member, callback absence, source/preflight, Native compilation, Normal transfer, allocation layout or general OO claim follows. Result pointer identity is not tested (the protocol supplies no comparison argument).

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `observed`. Channel: Original counted native ASCII String object vectors through Tcl_EvalObjv (flags0) or Jim_EvalObjVector; actual info patchlevel is a separate ASCII source query. Result bytes use the sole result getter. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

The original info commands vector returns an empty string. The fresh factory vector returns code1 with invalid command name "oo::configurable"; no class object creation vector is attempted.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted native ASCII String object vectors through Tcl_EvalObjv (flags0) or Jim_EvalObjVector; actual info patchlevel is a separate ASCII source query. Result bytes use the sole result getter. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

The original info commands vector returns ::oo::configurable. The original factory vector returns code0 with ::C, then the original class create vector returns code0 with ::o.

### bigip

Status: `not-tested`.

No appliance launch is attached.

## Consumer bindings

- [rust/tcl-vm/src/cmd_oo/native_property_counted_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_property_counted_tests.rs), `cmd_oo::native_property_counted_tests::original_configurable_availability_matches_native_counted_argv_without_source_preflight` (linked): Recreates authentic per-profile native core and exact original counted object vectors; compares availability/factory/class-create codes and bytes against this finite original-argv capture, while source preflight and C9 property access remain separate.

## Evidence

- [rust/tcl-registry/tests/data/native_property_availability_original](../../../../rust/tcl-registry/tests/data/native_property_availability_original)
- [rust/tcl-registry/tests/data/native_property_availability_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_property_availability_original/8.4.20)
- [rust/tcl-registry/tests/data/native_property_availability_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_property_availability_original/8.5.19)
- [rust/tcl-registry/tests/data/native_property_availability_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_property_availability_original/8.6.18)
- [rust/tcl-registry/tests/data/native_property_availability_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_property_availability_original/9.0.4)
- [rust/tcl-registry/tests/data/native_property_availability_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_property_availability_original/9.1.0)
- [rust/tcl-registry/tests/data/native_property_availability_original/jim](../../../../rust/tcl-registry/tests/data/native_property_availability_original/jim)
