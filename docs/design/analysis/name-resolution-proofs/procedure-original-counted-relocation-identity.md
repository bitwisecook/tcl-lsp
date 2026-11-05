# naming.procedure.original-counted-relocation-identity

Kind: `native-observation`

## Problem statement

A procedure name containing FF or counted zero can be moved using directly constructed name objects. Rendering or Unicode repair before rename can target a different command. This check observes the actual original command pointer and its future namespace, not a logical name roundtrip.

## Question

Do the four recorded counted FF/NUL procedure-name moves preserve the original command identity and select the destination namespace?

## Conclusion

All five C releases and the hashed Jim library record rename success, original command identity1, and future result ::B for all four counted-name controls. These observations establish only the selected rename/command identity within each process; they do not prove a general name comparator, resident string protocol or source Document producer.

## Scope

Exactly four counted FF/NUL name moves per engine. C releases and hashed Jim library are observed; Jim reported revision/patchlevel and executable hashes are not retained. No BIG-IP or Document-character-channel claim.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Direct counted Tcl_Obj/Jim_Obj names supplied to native rename calls; ASCII source setup, no character-channel conversion. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim Jim; patchlevel/revision not retained.

Four counted-name moves: rename code0, original command identity1, future call code0/result ::B.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/interp/native_procedure_relocation_tests.rs](../../../../runtime/rust/src/interp/native_procedure_relocation_tests.rs), `relocation_of_original_counted_names_matches_native_binding_identity` (linked): Compare the counted rename command identity and future namespace for all24 controls.

## Evidence

- [runtime/rust/tests/data/native_procedure_relocation/raw-manifest.json](../../../../runtime/rust/tests/data/native_procedure_relocation/raw-manifest.json)
- [runtime/rust/tests/data/native_procedure_relocation/raw-probe.c](../../../../runtime/rust/tests/data/native_procedure_relocation/raw-probe.c)
- [runtime/rust/tests/data/native_procedure_relocation/raw-observations.tsv](../../../../runtime/rust/tests/data/native_procedure_relocation/raw-observations.tsv)
