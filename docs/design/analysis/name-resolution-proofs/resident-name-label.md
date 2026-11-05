# naming.presentation.resident-name-label

Kind: `implementation-contract`

## Problem statement

Lossy Unicode rendering can merge invalid resident bytes, and printing literal escape sequences without quoting backslashes can make an ordinary name indistinguishable from an escaped opaque byte or zero. Readable labels must remain separate from source spelling and semantic keys.

## Question

Does the shared readonly resident-name label retain printable Unicode and distinct escaped control, backslash and invalid byte displays without supplying source, lookup or edit authority?

## Conclusion

The label walks exact resident byte chunks. Valid printable Unicode is readable; backslashes and controls are escaped, and each invalid byte is rendered explicitly instead of replaced or discarded. Fixed controls keep raw zero, encoded C080, distinct surrogate-byte sequences, FF and literal escape spellings separate. Workspace symbols consume this shared presenter after independently selecting and validating their original source owners. The returned String is presentation only; it cannot establish a naming recipe, source roundtrip, equality, lookup, runtime value or edit permission.

## Scope

Current pure Rust readonly byte presentation. Fixed inputs cover ordinary Unicode, backslashes, control zero, malformed native units and literal escaped text. No C, Jim or appliance observation is attached, and no passing Rust execution is inferred. Callers retain independent byte/policy identity and source currency.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This is a Rust source-consumer implementation contract. No native provider observation or executed Rust result is attached; selected source advice supplies no physical native or runtime authority.

## Consumer bindings

- [rust/tcl-syntax/src/native_string/name_display.rs](../../../../rust/tcl-syntax/src/native_string/name_display.rs), `resident_name_label`: Present complete resident bytes without replacing invalid units or treating the label as source spelling or semantic identity.
- [rust/tcl-syntax/src/native_string/name_display.rs](../../../../rust/tcl-syntax/src/native_string/name_display.rs), `native_string::name_display::tests::labels_preserve_counted_bytes_without_claiming_source_roundtrips` (linked): Printable Unicode stays readable while raw zero/C080/surrogate bytes/FF and literal backslash escapes retain distinct labels; no source roundtrip is claimed.
