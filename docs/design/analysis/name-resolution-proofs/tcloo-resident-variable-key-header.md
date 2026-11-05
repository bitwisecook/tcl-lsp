# naming.tcloo.resident-variable-key-header

Kind: `native-observation`

## Problem statement

A shortened public variable-name result does not show whether the object namespace actually stores a short key. The counted TclOO declaration, namespace hash key and report leaf can be different objects. These original-header snapshots distinguish retained counted keys from later API result conversion; they apply only to the captured current object/namespace records, not arbitrary header or compiler capabilities.

## Question

At the captured pre/post reporter windows, what are the existing declaration and object-namespace hash-key header lengths, resident bytes and equality-to-original flags?

## Conclusion

C8.6/C9.0/C9.1 snapshots retain the full counted raw-zero declaration and actual resident namespace key, including length6 and equality-to-original/stored-declaration flags, before and after the public reporter. C8.6 has a short legacy report leaf while its resident key remains counted. Qualification-after-zero controls do not gain the same namespace key. These are original header windows, not returned-list or native compiler authority.

## Scope

Six fixed declaration/primary arrangements (plain, raw zero, qualifier-after-zero, D800, FF, prefix negative), exact original v1..v5 inputs/observer chronology and API windows. C86/C90/C91 supported OO, C84/C85/Jim explicit unavailable OO controls. v3 C86 failed compile is retained separately.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `unsupported`. Channel: Configured private-header C snapshots of actual stored declaration and object namespace hash key before/after public reporter; no new getter in existing-byte/header window. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Captured OO_UNAVAILABLE control; no TclOO resident-key/API comparison executed for this provider.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Configured private-header C snapshots of actual stored declaration and object namespace hash key before/after public reporter; no new getter in existing-byte/header window. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Declaration and resident raw-zero key header length6 and full bytes 6b007461696c are retained with same-original/stored-declaration equality flags.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-vm/tests/data/native_oo_resident_keys/v4](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.4.20](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.4.20)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.5.19](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.5.19)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.6.18](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/8.6.18)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.0.4](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.0.4)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.1.0](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/9.1.0)
- [rust/tcl-vm/tests/data/native_oo_resident_keys/v4/jim](../../../../rust/tcl-vm/tests/data/native_oo_resident_keys/v4/jim)
