# naming.property.original-foundation-epoch-and-cache

Kind: `native-observation`

## Problem statement

An instance method change, a class method change and a structure change affect different property-cache owners. A class-definition error can occur after the mutation, so transaction-like rollback would preserve the wrong cache generation. This question follows the original cache and foundation epoch through those distinct operations.

## Question

Which recorded C9.1 method and structure changes advance the property foundation epoch or replace its cache?

## Conclusion

An instance method addition leaves epoch32 and the original cache intact. A class method is installed before a caught LATE error, advances the foundation to33, and leaves the old header until the next enumeration refreshes it. An unused class structure change leaves epoch33; changing the used class structure advances to34. The exact result does not license a broad class/object mutation rule.

## Scope

Only the recorded C Tcl9.1.0 process linked to the hashed private-header build is observed. Other C releases, Jim and BIG-IP are not tested for this question. Native object/cache/reference snapshots are distinct from source-name bytes, command dispatch and current runtime ownership. An independently retained full11-row property-name probe also supplies the same earlier scoped windows; its additional reader/writer duplicate rows belong to naming.property.original-accessor-name-cache-children. Exact whole probe and full-stream hashes are retained independently, not substituted as a duplicate capture.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim

Status: `not-tested`.

No observation for this question.

### tcl9.1

Status: `observed`. Channel: ASCII Tcl_Eval scripts, including caught guest Error after class-method installation; private epoch/cache snapshots. Version: tcl9.1 9.1.0.

instance-method epoch32/delta0/same header1; class-method-error epoch33/delta1/old header1; refresh header0; untangled structure delta0; used structure epoch34/delta1. Independent property-name probe rows2–6 exactly reproduce these earlier row bytes; its original full11-row input/stream remains separately retained.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `native_property_lookup::tests::property_temporary_lookup_and_mutation_axes_are_independent` (linked): Keep instance/foundation invalidation and failed-definition chronology independent.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `property_epochs_follow_mutations_before_definition_failure` (linked): Check class mutation is retained before definition Error.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `property_epochs_follow_mutations_before_definition_failure` (linked): Check the same mutation-before-Error boundary.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `cmd_oo::native_properties::tests::property_epochs_follow_mutations_before_definition_failure` (linked): Property epochs follow mutations before definition failure. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `cmd_oo::native_properties::tests::property_epochs_follow_mutations_before_definition_failure` (linked): Property epochs follow mutations before definition failure. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.

## Evidence

- [rust/tcl-registry/tests/data/native_property_owners/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_owners/manifest.json)
- [rust/tcl-registry/tests/data/native_property_owners/probe.c](../../../../rust/tcl-registry/tests/data/native_property_owners/probe.c)
- [rust/tcl-registry/tests/data/native_property_owners/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_owners/native.tsv)
- [rust/tcl-registry/tests/data/native_property_names/probe.c](../../../../rust/tcl-registry/tests/data/native_property_names/probe.c)
- [rust/tcl-registry/tests/data/native_property_names/manifest.json](../../../../rust/tcl-registry/tests/data/native_property_names/manifest.json)
- [rust/tcl-registry/tests/data/native_property_names/native.tsv](../../../../rust/tcl-registry/tests/data/native_property_names/native.tsv)
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs)
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs)
