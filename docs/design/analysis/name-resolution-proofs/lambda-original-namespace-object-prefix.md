# naming.lambda.original-namespace-object-prefix

Kind: `native-observation`

## Problem statement

Lambda namespace selection can be incorrectly inferred from source presentation or assumed identical to Procedure publication. Binary-produced values and raw counted String objects also require separate input channels before zero-byte behavior can be related.

## Question

What namespace current string or caught error does the exact source return for ten Apply namespace controls spanning plain/colon/root forms, binary-produced00 suffixes and literal UTF8 Unicode?

## Conclusion

C8.5–C9.1 return::ns for the plain/colon variants and:: for omitted/empty namespaces, while all binary-produced00 suffix cases fail full namespace lookup. Current Jim returns literal root-prefixed spellings and preserves complete00 suffixes. C8.4 cannot execute Apply. These source results do not establish raw counted String00 or C ByteArray getter behavior.

## Scope

Six fresh source-file CLI processes retain six queried versions and78 sequential caught controls:18 namespace setup catches and60 Apply attempts. Each process shares its namespace setup; the controls are separately caught rather than independently initialized. The exact probe contains literal UTF8 U+00E9 even though immutable request/receipt metadata calls it ASCII/escaped. Source-produced binary values and stdout encoding remain separate from sampled native object bytes. No namespace pointer/physical allocation, object/cache/header, command token, source/compiler entry, arbitrary Normal, frame correspondence or BIG-IP behavior is measured. The independent constructor/getter question supplies actual original SDK input objects and its own narrower representation samples. Independent linked implementation coverage: The shared C85–91 recipe retains original getter bytes and prefixes exact :: only when the counted object lacks that prefix. Namespace lookup applies its separate C-string extent afterwards; no actual namespace existence, object/header storage or entered frame follows from the pure byte projection. C84 and Jim refuse this C-specific purpose. Compiler explicit lambda namespace and child-source lambda advice, VM actual original getter and Runtime original cached/fallback lambda object producers consume the same helper. Jim retains its own counted original namespace object route.

## Provider answers

### tcl8.4

Status: `unsupported`. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls. Version: tcl8.4 8.4.20.

All three namespace setups succeed, but each of the ten Apply controls fails because apply is unavailable, with errorCode NONE. No lambda namespace selection or binary-produced namespace value acceptance is measured for this release.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Plain/relative/absolute colon spellings return::ns; omitted/empty namespaces return root::. All three binary-format-produced00 suffix variants fail namespace lookup with the complete suffix preserved in the diagnostic/errorCode. Literal UTF8 :é returns::é. These source-produced values do not establish raw counted original String00 behavior; the independent SDK constructor/getter question supplies that channel.

### jim

Status: `observed`. Channel: Exact UTF8 LF source file with literal U+00E9 in two source lines and no literal NUL; binary format produces the three00-containing values. One fresh CLI process shares namespace setup across thirteen sequential caught controls. Version: jim 0.84-9-g5bac7c9.

The exact returned namespace current strings retain a literal root prefix before the complete supplied namespace spelling: :ns→:::ns, :::ns→:::::ns, ns→::ns and::ns→::::ns. Omitted/empty namespaces return::. All three binary-produced00 suffix cases complete0 and preserve the full00/suffix in the reported namespace string. Literal UTF8 :é returns:::é. These Jim results are independent of C namespace allocation and constructor/getter behavior.

### bigip

Status: `not-tested`.

No appliance observation of these exact lambda namespace source controls.

## Consumer bindings

- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNamePurpose`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_input`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-syntax/src/naming/native.rs](../../../../rust/tcl-syntax/src/naming/native.rs), `NativeNameProtocol::lambda_namespace_path`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `namespace_for_lambda_operand`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-compiler/src/analyser/interp_visibility.rs](../../../../rust/tcl-compiler/src/analyser/interp_visibility.rs), `literal_lambda_body`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [rust/tcl-vm/src/command.rs](../../../../rust/tcl-vm/src/command.rs), `lambda_namespace`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.
- [runtime/rust/src/cmd_proc.rs](../../../../runtime/rust/src/cmd_proc.rs), `apply_native_c`: Retain the independently selected original lambda namespace input/getter route and its separate source or runtime purpose; linked Rust coverage does not promote native field samples or entered frame correspondence.

## Evidence

- [rust/tcl-registry/tests/data/native_lambda_namespace_source](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source)
- [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.6.18](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.6.18)
- [rust/tcl-registry/tests/data/native_lambda_namespace_source/9.1.0](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/9.1.0)
- [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.4.20](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.4.20)
- [rust/tcl-registry/tests/data/native_lambda_namespace_source/9.0.4](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/9.0.4)
- [rust/tcl-registry/tests/data/native_lambda_namespace_source/8.5.19](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/8.5.19)
- [rust/tcl-registry/tests/data/native_lambda_namespace_source/jim](../../../../rust/tcl-registry/tests/data/native_lambda_namespace_source/jim)
