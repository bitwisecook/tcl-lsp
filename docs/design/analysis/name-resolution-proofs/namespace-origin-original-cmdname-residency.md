# naming.namespace.origin-original-cmdname-residency

Kind: `native-observation`

## Problem statement

namespace origin may consume a name object that already has a cmdName cache, including one whose string representation has been invalidated. Reconstructing original source ownership from the returned origin or resident bytes would conflate distinct values. This direct object-vector probe retains the same original name across origin, rename and replacement operations.

## Question

Does namespace origin preserve the original cmdName primary and resident-string absence across the recorded C import operations?

## Conclusion

All five resident captures preserve original cmdName and resident bytes while results change from ::src::p to ::src::q to ::dest::p. C8.5–C9.1 stringless captures preserve cmdName and leave the original bytes absent after invalidation, returning ::src::p again. C8.4 has no retained stringless observation. The result spelling cannot donate original word/namespace/Native ownership.

## Scope

Only these nine C process snapshots are observed; C8.4 stringless, Jim and BIG-IP are not tested. Returned namespace origin, original primary and resident string are independent purposes.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Direct counted original Tcl_Obj passed to Tcl_EvalObjv; ASCII Tcl_Eval setup and explicit Tcl_InvalidateStringRep in the stringless variant. Version: tcl8.4 8.4.20.

Resident: cmdName remains, bytes present, results ::src::p / ::src::q / ::dest::p. Stringless mode not recorded.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Direct counted original Tcl_Obj passed to Tcl_EvalObjv; ASCII Tcl_Eval setup and explicit Tcl_InvalidateStringRep in the stringless variant. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Resident: cmdName remains, bytes present, results ::src::p / ::src::q / ::dest::p. Stringless: cmdName remains, original bytes absent after invalidation, result ::src::p.

### jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

- [runtime/rust/src/cmd_oo/original_object_resolution_tests.rs](../../../../runtime/rust/src/cmd_oo/original_object_resolution_tests.rs), `cmd_oo::original_object_resolution_tests::original_oo_lookup_keeps_stringless_command_and_import_token_roles` (linked): Oo lookup keeps stringless command and import token roles. This is a source-defined software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native chronology, physical reference-count/frame identity or execution permission.
- [runtime/rust/src/cmd_oo/original_object_resolution_tests.rs](../../../../runtime/rust/src/cmd_oo/original_object_resolution_tests.rs), `cmd_oo::original_object_resolution_tests::original_import_projection_never_substitutes_a_recreated_source_spelling` (linked): Import projection never substitutes a recreated source spelling. This is a source-defined software control without an attached matching current owned assertion outcome. It grants no external-provider observation, Native chronology, physical reference-count/frame identity or execution permission.

## Evidence

- [runtime/rust/tests/data/native_namespace_origin/original_provenance.json](../../../../runtime/rust/tests/data/native_namespace_origin/original_provenance.json)
- [runtime/rust/tests/data/native_namespace_origin/original_probe.c](../../../../runtime/rust/tests/data/native_namespace_origin/original_probe.c)
