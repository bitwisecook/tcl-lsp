# naming.source.native-string-original-extents

Kind: `implementation-contract`

## Problem statement

One retained native text component can contain both literal source units and multiple escaped units. Treating it as an indivisible component loses an editable tail; taking native byte offsets as source offsets can split an escape or a multi-byte source unit and corrupt the authored name.

## Question

Does original native-string extent projection preserve complete escaped units and original source-channel boundaries across a decoded text run?

## Conclusion

The shared mapper uses the selected native string protocol, full escape syntax and source channel to project only exact native unit boundaries into original spans. Complete escape-produced units map to their entire original escape, interior byte boundaries decline, and out-of-range extents decline. The Registry word adapter composes those spans with original grouping and word ownership. This correspondence grants no selected name identity, editable rename, compiler preparation, runtime value, normal completion or physical cache state.

## Scope

Pure Syntax/Registry original-source geometry for the five C Tcl native string recipes and the Jim recipe. Fixed tests cover escaped surrogate units, Document versus NativeValue zero ingress, braces/quotes and invalid extents. A selected protocol is required; general computed word or list-child editability is a separate owner.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This question specifies a Rust implementation contract; no native provider observation or Rust execution result is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs), `native_source_string_extent`: Projects decoded text through the existing native escape decoder and literal channel boundary owner.
- [rust/tcl-registry/src/native_compiler_words.rs](../../../../rust/tcl-registry/src/native_compiler_words.rs), `original_literal_extent`: Attaches shared literal/escaped unit extents to the original retained whole word.
- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs), `backslash::executable_text_tests::escaped_text_extents_keep_complete_escapes_and_channel_unit_boundaries` (linked): Checks complete surrogate escape extents, rejects interior and out-of-bounds native ranges, and preserves channel-specific zero-unit boundaries.
- [rust/tcl-registry/src/native_compiler_words.rs](../../../../rust/tcl-registry/src/native_compiler_words.rs), `native_compiler_words::tests::original_literal_extents_preserve_grouping_escapes_and_channel_units` (linked): Checks captured word grouping and escaped native ranges against the exact original source image.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named fixed Rust tests bind this implementation contract. Execute the corresponding crate selectors against one complete current source snapshot. This record supplies no Rust execution receipt, native test result, physical object, normal-completion certificate or cache proof.
