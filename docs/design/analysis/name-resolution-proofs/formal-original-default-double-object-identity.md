# naming.formal.original-default-double-object-identity

Kind: `native-observation`

## Problem statement

Parsing the outer formal List and inner name/default pair can reconstruct a default as String or retain the actual original Double. A later textual1.5 value cannot decide the bound object identity or its primary cache.

## Question

Does an omitted optional argument bind the actual original Double1.5 default object and primary?

## Conclusion

The actual callback observes a distinct String default in C8.4/8.5, and the same original Double default in C8.6–9.1 and Jim. These are direct argument pointer/type comparisons at the callback. Refcounts, arbitrary default formal topology, compiler locals and introspection preservation are not printed by this producer.

## Scope

One exact original Double1.5 lives in an inner name/default List, inside a real outer parameter List. Public object-vector proc definition and one-argument p call reach capture$arg; callback compares original pointer and primary before a string getter. Six original one-row files equal the manifest observation objects. This minimal receipt supplies release/library SHA/source SHA but no header/executable/compile/run status or launched patchlevel query. The preserved streams are valid evidence bytes; missing original whole-stream hashes/build fields remain limits. Jim label0.84 is capture association with unrecorded revision/configuration; no BIG-IP or Rust execution claim.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: Real original Double in nested formal Lists, public original argv definition/invocation and callback argument identity. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Actual callback reports {"same_object":false,"cache":"string"}. Original raw stream equals the attached manifest observation; missing process/build fields are not inferred.

### tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: Real original Double in nested formal Lists, public original argv definition/invocation and callback argument identity. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim Jim capture.

Actual callback reports {"same_object":true,"cache":"double"}. Original raw stream equals the attached manifest observation; missing process/build fields are not inferred.

### bigip

Status: `not-tested`.

No original observation for this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `parse_params_object`: Retains selected original default object ownership apart from its printed spelling.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `cmd_proc::tests::native_formals_keep_storage_keys_and_original_default_objects` (linked): Recreates genuine Double/List defaults and compares retained pointer identity to all six original rows; does not claim original callback launched during the Rust test.

## Evidence

- [rust/tcl-syntax/tests/data/native_formal_storage/default-object-probe.c](../../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-probe.c)
- [rust/tcl-syntax/tests/data/native_formal_storage/default-object-manifest.json](../../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-manifest.json)
- [rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.4.20.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.4.20.jsonl)
- [rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.5.19.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.5.19.jsonl)
- [rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.6.18.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-8.6.18.jsonl)
- [rust/tcl-syntax/tests/data/native_formal_storage/default-object-9.0.4.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-9.0.4.jsonl)
- [rust/tcl-syntax/tests/data/native_formal_storage/default-object-9.1.0.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-9.1.0.jsonl)
- [rust/tcl-syntax/tests/data/native_formal_storage/default-object-jim0.84.jsonl](../../../../rust/tcl-syntax/tests/data/native_formal_storage/default-object-jim0.84.jsonl)
