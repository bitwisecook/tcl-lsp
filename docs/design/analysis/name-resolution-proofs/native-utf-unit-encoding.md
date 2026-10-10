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

## Evidence

- `probe`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/probe.c`, SHA-256 `8939e3d7f17ae325b9a68b7a35e3cd8991488e0cd626cefbdbe87930169adcc4`. Public Unicode encoder and constructor inputs.
- `manifest`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/manifest.json`, SHA-256 `41dec5fd5bde299e3abbaa1e8df54831e529a957ec23d7de7d3520755c32b960`. 107 actual rows, provider library/executable/log hashes and constructor width refusals.
- `native-8.4.20`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/8.4.20.jsonl`, SHA-256 `1fa3427bc0628797ecc1b020c0f4e261279e3814b8cc1ee7819af43d1693cfa2`. Actual encoder/constructor byte and pre-getter storage rows.
- `fixture-8.4.20`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/8.4.20.tsv`, SHA-256 `50f21275b83dfa636c61511185ccd0422d99853b3eb7696bb00a00be3f25bade`. Selected semantic byte outputs for Rust tests.
- `native-8.5.19`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/8.5.19.jsonl`, SHA-256 `1fa3427bc0628797ecc1b020c0f4e261279e3814b8cc1ee7819af43d1693cfa2`. Actual encoder/constructor byte and pre-getter storage rows.
- `fixture-8.5.19`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/8.5.19.tsv`, SHA-256 `50f21275b83dfa636c61511185ccd0422d99853b3eb7696bb00a00be3f25bade`. Selected semantic byte outputs for Rust tests.
- `native-8.6.18`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/8.6.18.jsonl`, SHA-256 `1fa3427bc0628797ecc1b020c0f4e261279e3814b8cc1ee7819af43d1693cfa2`. Actual encoder/constructor byte and pre-getter storage rows.
- `fixture-8.6.18`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/8.6.18.tsv`, SHA-256 `50f21275b83dfa636c61511185ccd0422d99853b3eb7696bb00a00be3f25bade`. Selected semantic byte outputs for Rust tests.
- `native-9.0.4`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/9.0.4.jsonl`, SHA-256 `262ac9df36c0efdcd34a8680f7b8d5e5543a4e98454d9a8298a06aff88833afe`. Actual encoder/constructor byte and pre-getter storage rows.
- `fixture-9.0.4`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/9.0.4.tsv`, SHA-256 `ac0a84f94fdf2146ef959ed9cbbd66ca05894960ed9c3c8a36cc5fbcab2cedf8`. Selected semantic byte outputs for Rust tests.
- `native-9.1.0`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/9.1.0.jsonl`, SHA-256 `262ac9df36c0efdcd34a8680f7b8d5e5543a4e98454d9a8298a06aff88833afe`. Actual encoder/constructor byte and pre-getter storage rows.
- `fixture-9.1.0`: `rust/tcl-syntax/tests/data/native_tcl_utf/unicode_encoder/9.1.0.tsv`, SHA-256 `ac0a84f94fdf2146ef959ed9cbbd66ca05894960ed9c3c8a36cc5fbcab2cedf8`. Selected semantic byte outputs for Rust tests.

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
