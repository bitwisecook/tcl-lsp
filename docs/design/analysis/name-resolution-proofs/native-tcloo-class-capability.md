# naming.runtime.native-tcloo-class-capability

Kind: `implementation-contract`

## Problem statement

A native command or namespace row cannot establish an OO class role, lifecycle, method implementation or support topology. Foreign and stale rows must not select a bootstrap factory.

## Question

How does an actual backend expose its current OO allocations without reconstructing role authority from command names or Registry metadata?

## Conclusion

NativeCompilationEntry retains an independently optional actual OO inventory. VM bootstrap installs actual support and method allocations, stamps their roles at allocation, and captures current method/Foundation epochs, lifecycle and namespace ownership. original_oo_class selects only a unique current token/generation in a closed same-interpreter/same-epoch inventory with matching actual private namespace. This observation does not issue a handler, body, compiler, variable, observer or Normal capability.

## Scope

Current runtime API and VM allocator/table implementation. Source-only constructed NativeEntry fixtures retain None. Runtime standalone has no independent NativeEntry inventory issuer . Physical C bootstrap observations remain separate named questions. Own-method observations explicitly distinguish a visibility-only record from a Script, opaque or intrinsic body. Method lookup continues past a visibility-only record while retaining its independent export state; such a record cannot supply lifecycle definition introspection.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `native_oo` (implementation): [rust/tcl-runtime-api/src/native_oo.rs](../../../../rust/tcl-runtime-api/src/native_oo.rs). SHA-256 `ab324692f0a766620e35102e0141f77976bd0cac161471e34b72d84bc87ced3d`. Current independently issued backend observation implementation; Root final formatting may reissue this mutable source digest.
- `native_compilation` (implementation): [rust/tcl-runtime-api/src/native_compilation.rs](../../../../rust/tcl-runtime-api/src/native_compilation.rs). SHA-256 `4706c5992d0a4009a73bf02837d9c70ca06be3b50ea18dec7f575b993f4cf0a8`. Current independently issued backend observation implementation; Root final formatting may reissue this mutable source digest.
- `native_bootstrap` (implementation): [rust/tcl-vm/src/cmd_oo/native_bootstrap.rs](../../../../rust/tcl-vm/src/cmd_oo/native_bootstrap.rs). SHA-256 `90898f4372bc2db2302a7bb523a0703ad4bf9fa57d9e96fe1660c6d699e5aec8`. Current independently issued backend observation implementation; Root final formatting may reissue this mutable source digest.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-runtime-api/src/native_oo.rs](../../../../rust/tcl-runtime-api/src/native_oo.rs), `NativeCompilationEntry::original_oo_class`: Current actual OO command/allocation/namespace join, without body or compiler authority.
- [rust/tcl-vm/src/cmd_oo/native_bootstrap.rs](../../../../rust/tcl-vm/src/cmd_oo/native_bootstrap.rs), `capture`: Observe actual VM allocation/table/lifecycle/namespace ownership independently of labels.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `interp::native_oo_bootstrap_inventory_tests::original_oo_bootstrap_inventory_keeps_roles_methods_and_namespace_currency` (linked): Actual VM root/support roles and method/lifecycle/private-namespace relationships retain their own allocations; foreign interpreter, stale epoch and replaced command generation withdraw the class selection.

A named test is a coverage binding, not a claim that it executed.

## Replay

No Rust execution receipt is attached. Visibility-only observations retain an independent allocation/flags facet and cannot supply a callable body or lifecycle definition. Native outputs remain separately scoped empirical questions.
