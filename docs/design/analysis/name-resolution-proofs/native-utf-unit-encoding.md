# What exact bytes do Tcl_UniCharToUtf and Tcl_NewUnicodeObj produce for zero, lone/reversed/adjacent surrogates and a supplementary scalar in the retained controls?

Proof ID: `naming.native-utf.unit-encoding`

## Problem statement

Rust Unicode scalar encoding cannot represent a native lone surrogate or modified zero, and can combine native UTF-16 units that a C string owner keeps separate. The integer-character API also has a different width boundary from constructing a Unicode object. Confusing these operations corrupts numeric escapes and source-name keys.

## Question

What exact bytes do Tcl_UniCharToUtf and Tcl_NewUnicodeObj produce for zero, lone/reversed/adjacent surrogates and a supplementary scalar in the retained controls?

## Scope

Pinned canonical C8.4.20–9.1.0 public Unicode encoder/constructor calls; no interpreter eval, byte array, Jim ABI or BIG-IP observation.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | 21 native rows; width refusals 1. See exact byte observations. |
| tcl8.5 8.5.19 | observed | 21 native rows; width refusals 1. See exact byte observations. |
| tcl8.6 8.6.18 | observed | 21 native rows; width refusals 1. See exact byte observations. |
| tcl9.0 9.0.4 | observed | 22 native rows; width refusals 0. See exact byte observations. |
| tcl9.1 9.1.0 | observed | 22 native rows; width refusals 0. See exact byte observations. |
| jim not recorded | not-tested | No observation for this precise question. |
| bigip not recorded | not-tested | No observation for this precise question. |

## Conclusion

All tested C unit encoders use C080 for zero and retain admitted surrogate units. C8 Unicode objects have16-bit units and refuse the supplementary-unit constructor control; the integer-character API substitutes U+FFFD for that point. C9 admits the scalar and encodes four bytes. Adjacent surrogate units remain six bytes. This is a unit/constructor observation, not a character-channel ingress or resident-byte normalization rule.

## Source anchors

No interpreter-source anchor is asserted for this question; the retained probe and outcomes establish the narrow observation.

## Shared owners and tests

- `rust/tcl-syntax/src/native_tcl_utf.rs`: `NativeTclUtf::encode_units` — Retained native unit encoding.
- `rust/tcl-syntax/src/native_tcl_utf.rs`: `NativeTclUtf::encode_character` — Integer-character API encoding.
- `rust/tcl-syntax/src/native_tcl_utf.rs`: `NativeTclUtf::encode_unit` — Single native unit encoder.
- `rust/tcl-syntax/src/native_tcl_utf.rs`: `native_tcl_utf::tests::encoding_matches_every_selected_native_unit_control`. Compare seventy selected unit/character outputs to actual C fixtures.

Native output is evidence for the interpreter operation. Rust tests must independently pass to establish implementation correspondence.

## Reconfirmation

```text
cc -I<version-source>/generic rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/probe.c <version-source>/unix/libtcl<major.minor>.a -lm -ldl -lpthread -lz -o /tmp/native-utf-unit-probe
```

Build separately against each exact named C release, run /tmp/native-utf-unit-probe and compare parsed JSON rows with that release fixture. C8 supplementary constructor refusal is probe admission data, not a failed harness. Jim and BIG-IP have no observation for this C API question.
