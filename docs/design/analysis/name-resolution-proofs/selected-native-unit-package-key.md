# naming.package.selected-native-unit-key

Kind: `implementation-contract`

## Problem statement

A package name can contain raw zero, modified UTF zero, opaque units or a non-ASCII spelling. Using a display string or the caller source grammar as package identity can clip, collapse or reencode names under the wrong operation. Diagnostic required-package metadata also has an explicit authored producer rather than a source-word receipt.

## Question

Does NativePackageNameKey apply only the package extent selected by its retained naming policy and preserve native units and producer-policy identity?

## Conclusion

NativePackageNameKey stores the selected package bytes and complete NamePolicyProtocol. Its constructor accepts native units, applies the existing package purpose, and supplies no source geometry, physical package entry or successful-operation proof. ASCII Registry metadata matching is exact and case-sensitive.

## Scope

Rust implementation contract under the explicit contexts used by the linked tests. This record makes no C Tcl, Jim or BIG-IP observation claim, no Native entry claim and no successful evaluation claim.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_package.rs](../../../../rust/tcl-registry/src/native_package.rs), `from_native_units`: Project package identity through the independently retained package naming recipe.
- [rust/tcl-registry/src/native_package.rs](../../../../rust/tcl-registry/src/native_package.rs), `matches_ascii`: Match explicitly authored ASCII Registry metadata without a display conversion.
- [rust/tcl-registry/src/native_package.rs](../../../../rust/tcl-registry/src/native_package.rs), `native_package::tests::package_name_keys_keep_native_units_and_package_extent_separate` (linked): Keep raw-zero extent separate from modified zero, opaque units and distinct producer policies.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "--manifest-path",
  "rust/Cargo.toml",
  "native_package::tests::package_name_keys_keep_native_units_and_package_extent_separate"
]
```

Run each listed selector in its owning crate; the argument vector shows the first selector. A coverage binding is not a recorded test execution. Native interpreter measurements are separate records.
