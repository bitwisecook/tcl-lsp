# naming.property.original-c9-counted-name-access

Kind: `native-observation`

## Problem statement

Using UTF-8 presentation names for C9.0 properties merges opaque declaration and accessor keys even though native C9.0 and C9.1 share counted/CString name purposes. C9.1 physical property caches remain an independent purpose.

## Question

How do the finite original counted property declaration, option, accessor and stored value controls behave in the six selected providers?

## Conclusion

C9.0 and C9.1 preserve the four counted declaration operands and return the original stored list through default accessors. D800 and D801 custom getters remain distinct. The zero operand formats its roster/option with the CString -p extent, independently of original counted name retention. Both releases reject the six forbidden declaration names with the same recorded byte diagnostics. Older C and Jim never reach these property operations because configurable is unavailable. The independently inspected C9.1 GetImplNamesForProperty window formats accessor children from the member CString extent, including its already present dash. The pure member-accessor projection checks the four retained C9.1 member rows independently from declaration-name formatting; this source explanation does not establish a physical property cache capability.

## Scope

Finite pFF, pD800, pD801 and p/raw-zero/tail names; default original-list write/read identity, separate D800/D801 custom getters, six invalid names, and older C/Jim availability controls. Public original object-vector input is distinct from ASCII bootstrap/roster source. No physical property header, epoch, Index cache, object storage layout, unmeasured mutation release, observer closure, source installation/Normal, arbitrary property ordering or BIG-IP result is inferred.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `observed`. Channel: Original counted native String/list object vectors for declaration/configure; separate ASCII bootstrap, version and property roster source queries. Original result pointer is sampled before the sole result getter. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

The actual configurable command inventory is empty. All five fresh bootstrap source queries return code 1 with the recorded unavailable-command diagnostic. Property declaration/access and invalid-name controls are not reached; these are availability negatives.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Original counted native String/list object vectors for declaration/configure; separate ASCII bootstrap, version and property roster source queries. Original result pointer is sampled before the sole result getter. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

All four counted names pFF, pD800, pD801 and p/raw-zero/tail define, write and read successfully; each default read retains the original supplied list object. D800/D801 custom getters remain distinct as FIRST/SECOND. The zero roster is -p, preserving the independent CString formatting extent. All six invalid declarations return the recorded byte diagnostics.

### bigip

Status: `not-tested`.

No appliance execution is attached.

## Source inspection

tcl9.1 9.1.0, revision `Recorded retained tcl9.1.0 full source; source-control revision unrecorded.`, `tmp/tcl9.1.0/generic/tclOOProp.c`, function `GetImplNamesForProperty`, lines 165–184. Retained reference `member-accessor-source`.

```text
static PropertyName *
GetImplNamesForProperty(
    Tcl_Obj *objPtr)
{
    Tcl_ObjInternalRep *irPtr = TclFetchInternalRep(objPtr, &propNameType);
    if (irPtr) {
	return GET_PROPERTY_NAME(irPtr);
    }

    // No cached version; make it

    const char *propName = TclGetString(objPtr);
    PropertyName *namePtr = (PropertyName *) Tcl_Alloc(sizeof(PropertyName));
    namePtr->readerName = Tcl_ObjPrintf("<ReadProp%s>", propName);
    namePtr->writerName = Tcl_ObjPrintf("<WriteProp%s>", propName);
    Tcl_IncrRefCount(namePtr->readerName);
    Tcl_IncrRefCount(namePtr->writerName);
    SET_PROPERTY_NAME(objPtr, namePtr);
    return namePtr;
}

```

## Consumer bindings

- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `NativePropertyLookupProtocol::member_accessor_names`: Selects exact C91 already-dashed original configuration member CString formatting independently from the C90/C91 pure declaration-name formatter.
- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `native_property_lookup::tests::original_c9_name_recipe_matches_native_declaration_boundaries_without_cache_authority` (linked): Compares the six actual C9 declaration diagnostics and counted property projections; selects name recipes for C90/C91 while physical lookup stays C91-only, and older C/Jim/vendor purposes abstain.
- [rust/tcl-vm/src/cmd_oo/native_property_counted_tests.rs](../../../../rust/tcl-vm/src/cmd_oo/native_property_counted_tests.rs), `cmd_oo::native_property_counted_tests::original_property_names_and_values_match_native_c9_counted_access` (linked): Recreates actual C90/C91 core plus separately installed compiler, enters the exact bootstrap source channel, then compares original counted declaration/read/write/roster bytes, stored list identity, distinct opaque getters and invalid errors. Unavailable original-argv factory controls bind their independent naming.property.original-configurable-availability-counted-argv question; Native source refusal is not reinterpreted as a guest result.
- [runtime/rust/src/cmd_oo/native_property_counted_tests.rs](../../../../runtime/rust/src/cmd_oo/native_property_counted_tests.rs), `cmd_oo::native_property_counted_tests::original_property_names_and_values_match_native_c9_counted_access` (linked): Independently compares original Runtime result pointers before its only getter and the same finite counted name/argv controls with no host refusal. No Rust execution result is inferred.
- [rust/tcl-registry/src/native_property_lookup.rs](../../../../rust/tcl-registry/src/native_property_lookup.rs), `native_property_lookup::tests::original_c91_property_member_accessors_preserve_the_native_dash_once` (linked): Consumes the four actual C91 member roster/read/write rows, then joins cached dashed-member formatting to independent declaration-name formatting; CString zero extent is retained. No physical capability or execution result follows from the pure comparison.

## Evidence

- [rust/tcl-registry/tests/data/native_property_counted_original](../../../../rust/tcl-registry/tests/data/native_property_counted_original)
- [rust/tcl-registry/tests/data/native_property_counted_original/8.4.20](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.4.20)
- [rust/tcl-registry/tests/data/native_property_counted_original/8.5.19](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.5.19)
- [rust/tcl-registry/tests/data/native_property_counted_original/8.6.18](../../../../rust/tcl-registry/tests/data/native_property_counted_original/8.6.18)
- [rust/tcl-registry/tests/data/native_property_counted_original/9.0.4](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.0.4)
- [rust/tcl-registry/tests/data/native_property_counted_original/9.1.0](../../../../rust/tcl-registry/tests/data/native_property_counted_original/9.1.0)
- [rust/tcl-registry/tests/data/native_property_counted_original/jim](../../../../rust/tcl-registry/tests/data/native_property_counted_original/jim)
