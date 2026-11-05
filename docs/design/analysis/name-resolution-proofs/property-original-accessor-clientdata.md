# naming.property.original-accessor-clientdata

Kind: `native-observation`

## Problem statement

Default property accessors may retain the original declaration-name object as client data, rather than a separately rendered name. Confusing that ownership with the caller management option can change reference counts or cache the wrong object. The check uses direct counted name operands, including non-UTF8 and NUL, and inspects the exact C9.1 accessor objects.

## Question

Do default C9.1 property accessors retain the original counted declaration-name object and leave its primary representation unchanged?

## Conclusion

For p, xFF and x00tail, both generated accessor client-data pointers equal the original declaration object; it has three references and no typed primary at declaration, write and read. Default write returns empty; read returns the exact supplied VALUE object. The caller management option remains untyped. This does not establish general method-name equality or object ownership outside the captured accessor lifecycle.

## Scope

Only the recorded C Tcl9.1.0 process linked to the hashed private-header build is observed. Other C releases, Jim and BIG-IP are not tested for this question. Native object/cache/reference snapshots are distinct from source-name bytes, command dispatch and current runtime ownership.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, jim

Status: `not-tested`.

No observation for this question.

### tcl9.1

Status: `observed`. Channel: ASCII Tcl_Eval setup plus directly constructed counted Tcl_Obj names used by Tcl_EvalObjv flags0. Version: tcl9.1 9.1.0.

Three declaration/write/read windows: accessor clientData pointer equality 1/1, declaration refcount3/type none, empty write1, original read-value identity1 and caller primary none.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `default_property_methods_retain_original_clientdata_and_leave_its_primary` (linked): Compare accessor client-data identity, original declaration reference count/primary and caller cache separately.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `default_property_methods_retain_original_clientdata_and_leave_its_primary` (linked): Compare accessor client-data identity, original declaration reference count/primary and caller cache separately.
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs), `cmd_oo::native_properties::tests::default_property_methods_retain_original_clientdata_and_leave_its_primary` (linked): Default property methods retain original clientdata and leave its primary. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs), `cmd_oo::native_properties::tests::default_property_methods_retain_original_clientdata_and_leave_its_primary` (linked): Default property methods retain original clientdata and leave its primary. This is a current authored software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native callback/free chronology, physical reference/frame identity or execution purpose.

## Evidence

- [runtime/rust/tests/data/native_property_clientdata/manifest.json](../../../../runtime/rust/tests/data/native_property_clientdata/manifest.json)
- [runtime/rust/tests/data/native_property_clientdata/probe.c](../../../../runtime/rust/tests/data/native_property_clientdata/probe.c)
- [runtime/rust/tests/data/native_property_clientdata/native.tsv](../../../../runtime/rust/tests/data/native_property_clientdata/native.tsv)
- [runtime/rust/src/cmd_oo/native_properties.rs](../../../../runtime/rust/src/cmd_oo/native_properties.rs)
- [rust/tcl-vm/src/cmd_oo/native_properties.rs](../../../../rust/tcl-vm/src/cmd_oo/native_properties.rs)
